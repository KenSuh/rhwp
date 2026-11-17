import type { WasmBridge } from '@/core/wasm-bridge';
import { asInvalidEditHandleError, isInvalidEditHandleError, rawGateDoc, stateLabelOf, withHistorySuspended } from './history';
import { InvalidEditHandleError } from '@/core/wasm-bridge';
import type { DocumentPosition, CharProperties } from '@/core/types';

/** 편집 명령 공통 인터페이스 */
export interface EditCommand {
  readonly type: string;
  readonly timestamp: number;
  /** 명령 실행 — 실행 후 커서 위치 반환 */
  execute(wasm: WasmBridge): DocumentPosition;
  /** 역실행 — 실행 전 커서 위치 반환 */
  undo(wasm: WasmBridge): DocumentPosition;
  /** 연속 명령 병합 시도 */
  mergeWith(other: EditCommand, wasm?: WasmBridge): EditCommand | null;
  /** 리소스 해제 (스냅샷 명령의 메모리 반환 등). 스택에서 제거될 때 호출. */
  discard?(wasm: WasmBridge): void;
  /** recordWithoutExecute 전용: 네이티브 핸들/스냅샷에 봉인된 (undoLabel, redoLabel) 조회 */
  recordedLabels?(wasm: WasmBridge): { undoLabel: string; redoLabel: string } | null;
}

// ─── 편집 작업 서술자 (라우팅 통합) ────────────────────

/**
 * 편집 작업 서술자 — 호출부가 "무엇을 하려는가"만 기술하고,
 * 라우터(executeOperation)가 적절한 Undo 전략을 자동 선택한다.
 *
 * - command: 정밀 커맨드 (텍스트 삽입/삭제, 문단 분할/병합, 서식)
 * - snapshot: 스냅샷 기반 커맨드 (붙여넣기, 객체 삭제 등)
 * - record:  WASM 직접 호출 후 히스토리에만 기록 (IME, 객체 이동)
 */
export type OperationDescriptor =
  | { kind: 'command'; command: EditCommand }
  | { kind: 'snapshot'; operationType: string; operation: (wasm: WasmBridge) => DocumentPosition }
  | { kind: 'record'; command: EditCommand; beforeLabel?: string | null };

class TextEditInverse {
  private token: string | null = null;
  private undone = false;
  private after: DocumentPosition | null = null;

  run(wasm: WasmBridge, position: DocumentPosition, structural: boolean, operation: () => DocumentPosition): DocumentPosition {
    if (this.undone && this.token !== null && this.after !== null) {
      this.token = wasm.undoEdit(this.token);
      this.undone = false;
      return this.after;
    }
    this.token = wasm.beginTextEdit(position.sectionIndex, position.parentParaIndex ?? position.paragraphIndex, structural && !isCell(position));
    if (this.token === null) return operation();
    try {
      this.after = operation();
      wasm.finishTextEdit(this.token);
      return this.after;
    } catch (error) {
      wasm.finishTextEdit(this.token);
      wasm.discardEdit(wasm.undoEdit(this.token));
      this.token = null;
      throw error;
    }
  }

  undo(wasm: WasmBridge): void {
    if (this.token === null) throw new InvalidEditHandleError('INVALID_EDIT_HANDLE: inverse is unavailable');
    this.token = wasm.undoEdit(this.token);
    this.undone = true;
  }

  adopt(token: string, after: DocumentPosition): void {
    this.token = token;
    this.after = after;
  }

  labels(wasm: WasmBridge): { undoLabel: string; redoLabel: string } | null {
    if (this.token === null) return null;
    const raw = rawGateDoc(wasm)?.editHandleLabels?.(this.token);
    if (!raw) return null;
    return JSON.parse(raw) as { undoLabel: string; redoLabel: string };
  }

  merge(first: TextEditInverse, last: TextEditInverse, wasm: WasmBridge): boolean {
    if (first.token !== null && last.token !== null) {
      if (!wasm.mergeTextEdits(first.token, last.token)) return false;
      last.token = null;
    }
    this.token = first.token;
    this.after = last.after;
    first.token = null;
    last.discard(wasm);
    return true;
  }

  discard(wasm: WasmBridge): void {
    if (this.token !== null) wasm.discardEdit(this.token);
    this.token = null;
  }
}

// ─── 본문/셀 분기 헬퍼 ────────────────────────────────

function isCell(pos: DocumentPosition): boolean {
  return pos.parentParaIndex !== undefined;
}

/** 중첩 표(depth > 1)인지 확인 */
function isNestedCell(pos: DocumentPosition): boolean {
  return (pos.cellPath?.length ?? 0) > 1;
}

function cellParaIndexOf(pos: DocumentPosition): number {
  if (isNestedCell(pos)) {
    return pos.cellPath![pos.cellPath!.length - 1].cellParaIndex;
  }
  return pos.cellParaIndex!;
}

function sameNestedCellTarget(a: DocumentPosition, b: DocumentPosition): boolean {
  const ap = a.cellPath ?? [];
  const bp = b.cellPath ?? [];
  if (ap.length !== bp.length || ap.length === 0) return false;
  return ap.every((entry, index) => {
    const other = bp[index];
    if (!other) return false;
    if (entry.controlIndex !== other.controlIndex || entry.cellIndex !== other.cellIndex) return false;
    return index === ap.length - 1 || entry.cellParaIndex === other.cellParaIndex;
  });
}

/** cellPath를 WASM용 JSON 문자열로 변환 */
function cellPathJson(pos: DocumentPosition): string {
  return JSON.stringify(pos.cellPath ?? []);
}

function doInsertText(wasm: WasmBridge, pos: DocumentPosition, text: string): void {
  if (isNestedCell(pos)) {
    wasm.insertTextInCellByPath(pos.sectionIndex, pos.parentParaIndex!, cellPathJson(pos), pos.charOffset, text);
  } else if (isCell(pos)) {
    wasm.insertTextInCell(pos.sectionIndex, pos.parentParaIndex!, pos.controlIndex!, pos.cellIndex!, pos.cellParaIndex!, pos.charOffset, text);
  } else {
    wasm.insertText(pos.sectionIndex, pos.paragraphIndex, pos.charOffset, text);
  }
}

function doDeleteText(wasm: WasmBridge, pos: DocumentPosition, count: number): void {
  if (isNestedCell(pos)) {
    wasm.deleteTextInCellByPath(pos.sectionIndex, pos.parentParaIndex!, cellPathJson(pos), pos.charOffset, count);
  } else if (isCell(pos)) {
    wasm.deleteTextInCell(pos.sectionIndex, pos.parentParaIndex!, pos.controlIndex!, pos.cellIndex!, pos.cellParaIndex!, pos.charOffset, count);
  } else {
    wasm.deleteText(pos.sectionIndex, pos.paragraphIndex, pos.charOffset, count);
  }
}

function doGetTextRange(wasm: WasmBridge, pos: DocumentPosition, count: number): string {
  if (isNestedCell(pos)) {
    return wasm.getTextInCellByPath(pos.sectionIndex, pos.parentParaIndex!, cellPathJson(pos), pos.charOffset, count);
  } else if (isCell(pos)) {
    return wasm.getTextInCell(pos.sectionIndex, pos.parentParaIndex!, pos.controlIndex!, pos.cellIndex!, pos.cellParaIndex!, pos.charOffset, count);
  } else {
    return wasm.getTextRange(pos.sectionIndex, pos.paragraphIndex, pos.charOffset, count);
  }
}

function doGetParagraphLength(wasm: WasmBridge, pos: DocumentPosition): number {
  if (isNestedCell(pos)) {
    return wasm.getCellParagraphLengthByPath(pos.sectionIndex, pos.parentParaIndex!, cellPathJson(pos));
  } else if (isCell(pos)) {
    return wasm.getCellParagraphLength(pos.sectionIndex, pos.parentParaIndex!, pos.controlIndex!, pos.cellIndex!, pos.cellParaIndex!);
  }
  return wasm.getParagraphLength(pos.sectionIndex, pos.paragraphIndex);
}

function withCellParaIndex(pos: DocumentPosition, cellParaIndex: number): DocumentPosition {
  if (pos.cellPath?.length) {
    return {
      ...pos,
      cellParaIndex,
      cellPath: pos.cellPath!.map((entry, index) =>
        index === pos.cellPath!.length - 1 ? { ...entry, cellParaIndex } : entry,
      ),
    };
  }
  return { ...pos, cellParaIndex };
}

function doApplyCharFormat(wasm: WasmBridge, start: DocumentPosition, end: DocumentPosition, propsJson: string): void {
  if (isNestedCell(start)) {
    if (!sameNestedCellTarget(start, end)) {
      throw new Error('중첩 표의 서로 다른 셀에 걸친 글자 서식 적용은 셀 선택 모드에서 처리해야 합니다');
    }
    wasm.applyCharFormatInCellByPath(
      start.sectionIndex, start.parentParaIndex!, cellPathJson(start), start.charOffset, end.charOffset, propsJson,
    );
  } else if (isCell(start)) {
    wasm.applyCharFormatInCell(
      start.sectionIndex, start.parentParaIndex!, start.controlIndex!, start.cellIndex!, start.cellParaIndex!,
      start.charOffset, end.charOffset, propsJson,
    );
  } else {
    wasm.applyCharFormat(start.sectionIndex, start.paragraphIndex, start.charOffset, end.charOffset, propsJson);
  }
}

function deleteRangeInNestedCell(wasm: WasmBridge, start: DocumentPosition, end: DocumentPosition): void {
  if (!sameNestedCellTarget(start, end)) {
    throw new Error('중첩 표의 서로 다른 셀에 걸친 텍스트 삭제는 셀 선택 모드에서 처리해야 합니다');
  }
  const endPara = cellParaIndexOf(end);
  const doc = rawGateDoc(wasm);
  if (doc && typeof doc.deleteRangeInCellByPath === 'function') {
    doc.deleteRangeInCellByPath(
      start.sectionIndex,
      start.parentParaIndex!,
      cellPathJson(start),
      start.charOffset,
      endPara,
      end.charOffset,
    );
    return;
  }
  const startPara = cellParaIndexOf(start);
  if (startPara === endPara) {
    doDeleteText(wasm, start, Math.max(0, end.charOffset - start.charOffset));
    return;
  }

  const endPos = withCellParaIndex(end, endPara);
  const endLen = doGetParagraphLength(wasm, endPos);
  const endDeleteCount = Math.min(end.charOffset, endLen);
  if (endDeleteCount > 0) {
    doDeleteText(wasm, { ...endPos, charOffset: 0 }, endDeleteCount);
  }

  for (let p = startPara + 1; p < endPara; p += 1) {
    const paraPos = withCellParaIndex(start, p);
    const len = doGetParagraphLength(wasm, paraPos);
    if (len > 0) {
      doDeleteText(wasm, { ...paraPos, charOffset: 0 }, len);
    }
  }

  const startPos = withCellParaIndex(start, startPara);
  const startLen = doGetParagraphLength(wasm, startPos);
  if (startLen > start.charOffset) {
    doDeleteText(wasm, start, startLen - start.charOffset);
  }

  // 뒤 문단부터 병합해야 paragraph index가 앞으로 당겨져도 경로가 안정적이다.
  for (let p = endPara; p > startPara; p -= 1) {
    const mergePos = withCellParaIndex(start, p);
    wasm.mergeParagraphInCellByPath(mergePos.sectionIndex, mergePos.parentParaIndex!, cellPathJson(mergePos));
  }
}

// ─── 텍스트 삽입 명령 ─────────────────────────────────

export class InsertTextCommand implements EditCommand {
  private readonly inverse = new TextEditInverse();

  readonly type = 'insertText';
  readonly timestamp: number;

  constructor(
    private position: DocumentPosition,
    private text: string,
    timestamp?: number,
  ) {
    this.timestamp = timestamp ?? Date.now();
  }

  static recorded(position: DocumentPosition, text: string, token: string): InsertTextCommand {
    const command = new InsertTextCommand(position, text);
    command.inverse.adopt(token, { ...position, charOffset: position.charOffset + text.length });
    return command;
  }

  execute(wasm: WasmBridge): DocumentPosition {
    return this.inverse.run(wasm, this.position, false, () => {
      doInsertText(wasm, this.position, this.text);
      return { ...this.position, charOffset: this.position.charOffset + this.text.length };
    });
  }

  undo(wasm: WasmBridge): DocumentPosition {
    this.inverse.undo(wasm);
    return { ...this.position };
  }

  discard(wasm: WasmBridge): void { this.inverse.discard(wasm); }

  recordedLabels(wasm: WasmBridge): { undoLabel: string; redoLabel: string } | null {
    return this.inverse.labels(wasm);
  }

  mergeWith(other: EditCommand, wasm?: WasmBridge): EditCommand | null {
    if (!wasm) return null;
    if (!(other instanceof InsertTextCommand)) return null;
    if ((isNestedCell(this.position) || isNestedCell(other.position)) && !sameNestedCellTarget(this.position, other.position)) return null;
    // 같은 문단/셀인지 확인
    if (other.position.sectionIndex !== this.position.sectionIndex) return null;
    if (other.position.paragraphIndex !== this.position.paragraphIndex) return null;
    if (isCell(this.position) !== isCell(other.position)) return null;
    if (isCell(this.position)) {
      if (other.position.parentParaIndex !== this.position.parentParaIndex) return null;
      if (other.position.controlIndex !== this.position.controlIndex) return null;
      if (other.position.cellIndex !== this.position.cellIndex) return null;
      if (other.position.cellParaIndex !== this.position.cellParaIndex) return null;
    }
    // 연속 위치 확인
    const expectedOffset = this.position.charOffset + this.text.length;
    if (other.position.charOffset !== expectedOffset) return null;
    // 300ms 이내
    if (other.timestamp - this.timestamp > 300) return null;
    // 줄바꿈/탭 포함 시 병합 불가
    if (other.text.includes('\n') || other.text.includes('\t')) return null;

    const merged = new InsertTextCommand(this.position, this.text + other.text, this.timestamp);
    if (!merged.inverse.merge(this.inverse, other.inverse, wasm)) return null;
    return merged;
  }
}

// ─── 텍스트 삭제 명령 ─────────────────────────────────

export class DeleteTextCommand implements EditCommand {
  private readonly inverse = new TextEditInverse();

  readonly type = 'deleteText';
  readonly timestamp: number;

  /** undo용 삭제된 텍스트 (execute 시 보존) */
  private deletedText: string;

  constructor(
    private position: DocumentPosition,
    private count: number,
    private direction: 'forward' | 'backward',
    deletedText?: string,
    timestamp?: number,
  ) {
    this.deletedText = deletedText ?? '';
    this.timestamp = timestamp ?? Date.now();
  }

  execute(wasm: WasmBridge): DocumentPosition {
    return this.inverse.run(wasm, this.position, false, () => {
      // 삭제 전 텍스트 보존
      if (!this.deletedText) {
        this.deletedText = doGetTextRange(wasm, this.position, this.count);
      }
      doDeleteText(wasm, this.position, this.count);
      return { ...this.position };
    });
  }

  undo(wasm: WasmBridge): DocumentPosition {
    this.inverse.undo(wasm);
    return { ...this.position, charOffset: this.position.charOffset + this.deletedText.length };
  }

  discard(wasm: WasmBridge): void { this.inverse.discard(wasm); }

  recordedLabels(wasm: WasmBridge): { undoLabel: string; redoLabel: string } | null {
    return this.inverse.labels(wasm);
  }

  mergeWith(other: EditCommand, wasm?: WasmBridge): EditCommand | null {
    if (!wasm) return null;
    if (!(other instanceof DeleteTextCommand)) return null;
    if ((isNestedCell(this.position) || isNestedCell(other.position)) && !sameNestedCellTarget(this.position, other.position)) return null;
    if (other.direction !== this.direction) return null;
    if (other.timestamp - this.timestamp > 300) return null;
    // 같은 문단/셀 확인
    if (other.position.sectionIndex !== this.position.sectionIndex) return null;
    if (other.position.paragraphIndex !== this.position.paragraphIndex) return null;
    if (isCell(this.position) !== isCell(other.position)) return null;
    if (isCell(this.position)) {
      if (other.position.parentParaIndex !== this.position.parentParaIndex) return null;
      if (other.position.controlIndex !== this.position.controlIndex) return null;
      if (other.position.cellIndex !== this.position.cellIndex) return null;
      if (other.position.cellParaIndex !== this.position.cellParaIndex) return null;
    }

    if (this.direction === 'backward') {
      // Backspace: 연속 앞쪽 삭제
      if (other.position.charOffset === this.position.charOffset - other.count) {
        const merged = new DeleteTextCommand(
          other.position, this.count + other.count, 'backward',
          other.deletedText + this.deletedText, this.timestamp,
        );
        if (!merged.inverse.merge(this.inverse, other.inverse, wasm)) return null;
        return merged;
      }
    } else {
      // Delete: 같은 위치에서 연속 삭제
      if (other.position.charOffset === this.position.charOffset) {
        const merged = new DeleteTextCommand(
          this.position, this.count + other.count, 'forward',
          this.deletedText + other.deletedText, this.timestamp,
        );
        if (!merged.inverse.merge(this.inverse, other.inverse, wasm)) return null;
        return merged;
      }
    }
    return null;
  }
}

// ─── 강제 줄바꿈 명령 (Shift+Enter) ─────────────────────

export class InsertLineBreakCommand implements EditCommand {
  private readonly inverse = new TextEditInverse();

  readonly type = 'insertLineBreak';
  readonly timestamp = Date.now();

  constructor(private position: DocumentPosition) {}

  execute(wasm: WasmBridge): DocumentPosition {
    return this.inverse.run(wasm, this.position, false, () => {
      doInsertText(wasm, this.position, '\n');
      const newPos = { ...this.position, charOffset: this.position.charOffset + 1 };
      return newPos;
    });
  }

  undo(wasm: WasmBridge): DocumentPosition {
    this.inverse.undo(wasm);
    return { ...this.position };
  }

  discard(wasm: WasmBridge): void { this.inverse.discard(wasm); }

  mergeWith(): null { return null; }
}

// ─── 탭 삽입 명령 (Tab) ──────────────────────────────

export class InsertTabCommand implements EditCommand {
  private readonly inverse = new TextEditInverse();

  readonly type = 'insertTab';
  readonly timestamp = Date.now();

  constructor(private position: DocumentPosition) {}

  execute(wasm: WasmBridge): DocumentPosition {
    return this.inverse.run(wasm, this.position, false, () => {
      doInsertText(wasm, this.position, '\t');
      const newPos = { ...this.position, charOffset: this.position.charOffset + 1 };
      return newPos;
    });
  }

  undo(wasm: WasmBridge): DocumentPosition {
    this.inverse.undo(wasm);
    return { ...this.position };
  }

  discard(wasm: WasmBridge): void { this.inverse.discard(wasm); }

  mergeWith(): null { return null; }
}

// ─── 문단 분할 명령 (Enter) ───────────────────────────

export class SplitParagraphCommand implements EditCommand {
  private readonly inverse = new TextEditInverse();

  readonly type = 'splitParagraph';
  readonly timestamp = Date.now();

  constructor(private position: DocumentPosition) {}

  execute(wasm: WasmBridge): DocumentPosition {
    return this.inverse.run(wasm, this.position, true, () => {
      const { sectionIndex: sec, paragraphIndex: para, charOffset } = this.position;
      const result = JSON.parse(wasm.splitParagraph(sec, para, charOffset));
      if (result.ok) {
        return { sectionIndex: sec, paragraphIndex: result.paraIdx, charOffset: 0 };
      }
      return this.position;
    });
  }

  undo(wasm: WasmBridge): DocumentPosition {
    this.inverse.undo(wasm);
    return { ...this.position };
  }

  discard(wasm: WasmBridge): void { this.inverse.discard(wasm); }

  mergeWith(): null { return null; }
}

// ─── 문단 병합 명령 (문단 시작에서 Backspace) ─────────

export class MergeParagraphCommand implements EditCommand {
  private readonly inverse = new TextEditInverse();

  readonly type = 'mergeParagraph';
  readonly timestamp = Date.now();

  /** undo 시 분할 위치 (이전 문단의 원래 길이) */
  private mergePointOffset = 0;

  constructor(private position: DocumentPosition) {}

  execute(wasm: WasmBridge): DocumentPosition {
    return this.inverse.run(wasm, this.position, true, () => {
      const { sectionIndex: sec, paragraphIndex: para } = this.position;
      // 병합 전 이전 문단 길이 기억
      this.mergePointOffset = wasm.getParagraphLength(sec, para - 1);
      wasm.mergeParagraph(sec, para);
      return { sectionIndex: sec, paragraphIndex: para - 1, charOffset: this.mergePointOffset };
    });
  }

  undo(wasm: WasmBridge): DocumentPosition {
    this.inverse.undo(wasm);
    return { ...this.position };
  }

  discard(wasm: WasmBridge): void { this.inverse.discard(wasm); }

  mergeWith(): null { return null; }
}

// ─── 선택 영역 삭제 명령 ─────────────────────────────

export class DeleteSelectionCommand implements EditCommand {
  private readonly inverse = new TextEditInverse();

  readonly type = 'deleteSelection';
  readonly timestamp = Date.now();

  constructor(
    private start: DocumentPosition,
    private end: DocumentPosition,
  ) {}

  execute(wasm: WasmBridge): DocumentPosition {
    return this.inverse.run(wasm, this.start, true, () => {
      const { start, end } = this;

      if (isCell(start)) {
        const sec = start.sectionIndex;
        const startPara = cellParaIndexOf(start);
        const endPara = cellParaIndexOf(end);
        if (isNestedCell(start)) {
          deleteRangeInNestedCell(wasm, start, end);
        } else {
          wasm.deleteRangeInCell(sec, start.parentParaIndex!, start.controlIndex!, start.cellIndex!, startPara, start.charOffset, endPara, end.charOffset);
        }
      } else {
        const sec = start.sectionIndex;
        wasm.deleteRange(sec, start.paragraphIndex, start.charOffset, end.paragraphIndex, end.charOffset);
      }

      return { ...start };
    });
  }

  undo(wasm: WasmBridge): DocumentPosition {
    this.inverse.undo(wasm);
    return { ...this.end };
  }

  discard(wasm: WasmBridge): void { this.inverse.discard(wasm); }

  mergeWith(): null { return null; }
}

// ─── 글자 서식 적용 명령 ─────────────────────────────

/**
 * 글자 서식 적용 명령.
 *
 * 서식은 글자 모양 구간뿐 아니라 줄 배치(LineSeg 높이·첫 줄 vpos·태그)와 DocInfo
 * (글자 모양·글꼴·테두리 목록, 원본 스트림)까지 바꾼다. 손으로 쓴 charShapeId
 * 부분 복원은 이 부수효과를 되돌리지 못한 채 이전 라벨을 발행했으므로(R7 A),
 * 역연산은 실행 전후 문서 스냅샷 쌍으로 정확히 되돌리고 라벨은 그 쌍이 봉인한
 * 값으로만 발행한다.
 */
export class ApplyCharFormatCommand implements EditCommand {
  readonly type = 'applyCharFormat';
  readonly timestamp = Date.now();

  private readonly capture: SnapshotCommand;

  constructor(
    private start: DocumentPosition,
    private end: DocumentPosition,
    private props: Partial<CharProperties>,
  ) {
    this.capture = new SnapshotCommand('applyCharFormat', start, start, wasm => this.applyFormat(wasm));
  }

  execute(wasm: WasmBridge): DocumentPosition {
    return this.capture.execute(wasm);
  }

  undo(wasm: WasmBridge): DocumentPosition {
    return this.capture.undo(wasm);
  }

  discard(wasm: WasmBridge): void { this.capture.discard(wasm); }

  mergeWith(): null { return null; }

  private applyFormat(wasm: WasmBridge): DocumentPosition {
    const { start, end } = this;
    const propsJson = JSON.stringify(this.props);

    if (isCell(start)) {
      const startPara = cellParaIndexOf(start);
      const endPara = cellParaIndexOf(end);
      for (let p = startPara; p <= endPara; p++) {
        const paraStart = withCellParaIndex(start, p);
        const paraEnd = withCellParaIndex(end, p);
        const from = p === startPara ? start.charOffset : 0;
        const to = p === endPara ? end.charOffset : doGetParagraphLength(wasm, paraStart);
        if (to <= from) continue;
        doApplyCharFormat(
          wasm,
          { ...paraStart, charOffset: from },
          { ...paraEnd, charOffset: to },
          propsJson,
        );
      }
    } else {
      const sec = start.sectionIndex;
      const startPara = start.paragraphIndex;
      const endPara = end.paragraphIndex;
      for (let p = startPara; p <= endPara; p++) {
        const from = p === startPara ? start.charOffset : 0;
        const to = p === endPara ? end.charOffset : wasm.getParagraphLength(sec, p);
        if (to <= from) continue;
        wasm.applyCharFormat(sec, p, from, to, propsJson);
      }
    }

    return { ...this.start };
  }
}

// ─── 문단 끝에서 Delete로 다음 문단 병합 ─────────────

export class MergeNextParagraphCommand implements EditCommand {
  private readonly inverse = new TextEditInverse();

  readonly type = 'mergeNextParagraph';
  readonly timestamp = Date.now();

  constructor(private position: DocumentPosition) {}

  execute(wasm: WasmBridge): DocumentPosition {
    return this.inverse.run(wasm, this.position, true, () => {
      const { sectionIndex: sec, paragraphIndex: para } = this.position;
      wasm.mergeParagraph(sec, para + 1);
      return { ...this.position };
    });
  }

  undo(wasm: WasmBridge): DocumentPosition {
    this.inverse.undo(wasm);
    return { ...this.position };
  }

  discard(wasm: WasmBridge): void { this.inverse.discard(wasm); }

  mergeWith(): null { return null; }
}

// ─── 셀 내부 문단 분할 명령 (셀 내 Enter) ──────────────

export class SplitParagraphInCellCommand implements EditCommand {
  private readonly inverse = new TextEditInverse();

  readonly type = 'splitParagraphInCell';
  readonly timestamp = Date.now();

  constructor(private position: DocumentPosition) {}

  execute(wasm: WasmBridge): DocumentPosition {
    return this.inverse.run(wasm, this.position, true, () => {
      const pos = this.position;
      const sec = pos.sectionIndex;
      const ppi = pos.parentParaIndex!;
      const cpi = cellParaIndexOf(pos);
      if (isNestedCell(pos)) {
        wasm.splitParagraphInCellByPath(sec, ppi, cellPathJson(pos), pos.charOffset);
      } else {
        wasm.splitParagraphInCell(sec, ppi, pos.controlIndex!, pos.cellIndex!, cpi, pos.charOffset);
      }
      const nextCellParaIndex = cpi + 1;
      return {
        ...withCellParaIndex(pos, nextCellParaIndex),
        charOffset: 0,
      };
    });
  }

  undo(wasm: WasmBridge): DocumentPosition {
    this.inverse.undo(wasm);
    return { ...this.position };
  }

  discard(wasm: WasmBridge): void { this.inverse.discard(wasm); }

  mergeWith(): null { return null; }
}

// ─── 셀 내부 문단 병합 명령 (셀 문단 시작에서 Backspace) ──

export class MergeParagraphInCellCommand implements EditCommand {
  private readonly inverse = new TextEditInverse();

  readonly type = 'mergeParagraphInCell';
  readonly timestamp = Date.now();

  private mergePointOffset = 0;

  constructor(private position: DocumentPosition) {}

  execute(wasm: WasmBridge): DocumentPosition {
    return this.inverse.run(wasm, this.position, true, () => {
      const pos = this.position;
      const sec = pos.sectionIndex;
      const ppi = pos.parentParaIndex!;
      const cpi = cellParaIndexOf(pos);
      // 병합 전 이전 셀 문단 길이 기억
      this.mergePointOffset = doGetParagraphLength(wasm, withCellParaIndex(pos, cpi - 1));
      if (isNestedCell(pos)) {
        wasm.mergeParagraphInCellByPath(sec, ppi, cellPathJson(pos));
      } else {
        wasm.mergeParagraphInCell(sec, ppi, pos.controlIndex!, pos.cellIndex!, cpi);
      }
      return { ...withCellParaIndex(pos, cpi - 1), charOffset: this.mergePointOffset };
    });
  }

  undo(wasm: WasmBridge): DocumentPosition {
    this.inverse.undo(wasm);
    return { ...this.position };
  }

  discard(wasm: WasmBridge): void { this.inverse.discard(wasm); }

  mergeWith(): null { return null; }
}

// ─── 셀 내부 다음 문단 병합 명령 (셀 문단 끝에서 Delete) ──

export class MergeNextParagraphInCellCommand implements EditCommand {
  private readonly inverse = new TextEditInverse();

  readonly type = 'mergeNextParagraphInCell';
  readonly timestamp = Date.now();

  constructor(private position: DocumentPosition) {}

  execute(wasm: WasmBridge): DocumentPosition {
    return this.inverse.run(wasm, this.position, true, () => {
      const pos = this.position;
      const sec = pos.sectionIndex;
      const ppi = pos.parentParaIndex!;
      const cpi = cellParaIndexOf(pos);
      if (isNestedCell(pos)) {
        const nextPath = [...pos.cellPath!];
        nextPath[nextPath.length - 1] = { ...nextPath[nextPath.length - 1], cellParaIndex: cpi + 1 };
        wasm.mergeParagraphInCellByPath(sec, ppi, JSON.stringify(nextPath));
      } else {
        wasm.mergeParagraphInCell(sec, ppi, pos.controlIndex!, pos.cellIndex!, cpi + 1);
      }
      return { ...pos };
    });
  }

  undo(wasm: WasmBridge): DocumentPosition {
    this.inverse.undo(wasm);
    return { ...this.position };
  }

  discard(wasm: WasmBridge): void { this.inverse.discard(wasm); }

  mergeWith(): null { return null; }
}

// ─── 표 이동 명령 ─────────────────────────────────────

export class MoveTableCommand implements EditCommand {
  readonly type = 'moveTable';
  readonly timestamp: number;

  private resultPpi: number;
  private resultCi: number;

  constructor(
    private sec: number,
    private ppi: number,
    private ci: number,
    private deltaH: number,
    private deltaV: number,
    resultPpi: number,
    resultCi: number,
    timestamp?: number,
  ) {
    this.resultPpi = resultPpi;
    this.resultCi = resultCi;
    this.timestamp = timestamp ?? Date.now();
  }

  execute(wasm: WasmBridge): DocumentPosition {
    const result = wasm.moveTableOffset(this.sec, this.ppi, this.ci, this.deltaH, this.deltaV);
    this.resultPpi = result.ppi;
    this.resultCi = result.ci;
    return { sectionIndex: this.sec, paragraphIndex: this.resultPpi, charOffset: 0 };
  }

  undo(wasm: WasmBridge): DocumentPosition {
    wasm.moveTableOffset(this.sec, this.resultPpi, this.resultCi, -this.deltaH, -this.deltaV);
    return { sectionIndex: this.sec, paragraphIndex: this.ppi, charOffset: 0 };
  }

  mergeWith(other: EditCommand): EditCommand | null {
    if (!(other instanceof MoveTableCommand)) return null;
    if (other.sec !== this.sec) return null;
    // 연속 이동: 이전 결과 위치 == 다음 시작 위치
    if (other.ppi !== this.resultPpi || other.ci !== this.resultCi) return null;
    if (other.timestamp - this.timestamp > 500) return null;

    return new MoveTableCommand(
      this.sec, this.ppi, this.ci,
      this.deltaH + other.deltaH,
      this.deltaV + other.deltaV,
      other.resultPpi, other.resultCi,
      this.timestamp,
    );
  }
}

// ─── 그림 이동 명령 ─────────────────────────────────────

export class MovePictureCommand implements EditCommand {
  readonly type = 'movePicture';
  readonly timestamp: number;

  constructor(
    private sec: number,
    private ppi: number,
    private ci: number,
    private deltaH: number,
    private deltaV: number,
    private origHorzOffset: number,
    private origVertOffset: number,
    timestamp?: number,
  ) {
    this.timestamp = timestamp ?? Date.now();
  }

  execute(wasm: WasmBridge): DocumentPosition {
    const props = wasm.getPictureProperties(this.sec, this.ppi, this.ci);
    wasm.setPictureProperties(this.sec, this.ppi, this.ci, {
      horzOffset: ((props.horzOffset + this.deltaH) >>> 0),
      vertOffset: ((props.vertOffset + this.deltaV) >>> 0),
    });
    return { sectionIndex: this.sec, paragraphIndex: this.ppi, charOffset: 0 };
  }

  undo(wasm: WasmBridge): DocumentPosition {
    wasm.setPictureProperties(this.sec, this.ppi, this.ci, {
      horzOffset: this.origHorzOffset,
      vertOffset: this.origVertOffset,
    });
    return { sectionIndex: this.sec, paragraphIndex: this.ppi, charOffset: 0 };
  }

  mergeWith(other: EditCommand): EditCommand | null {
    if (!(other instanceof MovePictureCommand)) return null;
    if (other.sec !== this.sec || other.ppi !== this.ppi || other.ci !== this.ci) return null;
    if (other.timestamp - this.timestamp > 500) return null;

    return new MovePictureCommand(
      this.sec, this.ppi, this.ci,
      this.deltaH + other.deltaH,
      this.deltaV + other.deltaV,
      this.origHorzOffset,
      this.origVertOffset,
      this.timestamp,
    );
  }
}

export class MoveShapeCommand implements EditCommand {
  readonly type = 'moveShape';
  readonly timestamp: number;

  constructor(
    private sec: number,
    private ppi: number,
    private ci: number,
    private deltaH: number,
    private deltaV: number,
    private origHorzOffset: number,
    private origVertOffset: number,
    timestamp?: number,
  ) {
    this.timestamp = timestamp ?? Date.now();
  }

  execute(wasm: WasmBridge): DocumentPosition {
    const props = wasm.getShapeProperties(this.sec, this.ppi, this.ci);
    wasm.setShapeProperties(this.sec, this.ppi, this.ci, {
      horzOffset: ((props.horzOffset + this.deltaH) >>> 0),
      vertOffset: ((props.vertOffset + this.deltaV) >>> 0),
    });
    return { sectionIndex: this.sec, paragraphIndex: this.ppi, charOffset: 0 };
  }

  undo(wasm: WasmBridge): DocumentPosition {
    wasm.setShapeProperties(this.sec, this.ppi, this.ci, {
      horzOffset: this.origHorzOffset,
      vertOffset: this.origVertOffset,
    });
    return { sectionIndex: this.sec, paragraphIndex: this.ppi, charOffset: 0 };
  }

  mergeWith(other: EditCommand): EditCommand | null {
    if (!(other instanceof MoveShapeCommand)) return null;
    if (other.sec !== this.sec || other.ppi !== this.ppi || other.ci !== this.ci) return null;
    if (other.timestamp - this.timestamp > 500) return null;

    return new MoveShapeCommand(
      this.sec, this.ppi, this.ci,
      this.deltaH + other.deltaH,
      this.deltaV + other.deltaV,
      this.origHorzOffset,
      this.origVertOffset,
      this.timestamp,
    );
  }
}

// ─── 스냅샷 기반 명령 (복잡한 작업의 Undo/Redo) ─────

/**
 * Document 스냅샷을 이용한 Undo/Redo 명령.
 *
 * 역연산 구현이 복잡한 작업(붙여넣기, 객체 삭제 등)에 사용한다.
 * - 최초 실행: before 스냅샷 저장 → 작업 수행 → after 스냅샷 저장
 * - Undo: before 스냅샷으로 복원
 * - Redo: after 스냅샷으로 복원
 * - Discard: 양쪽 스냅샷 메모리 해제
 */
export class SnapshotCommand implements EditCommand {
  readonly type: string;
  readonly timestamp = Date.now();

  private beforeId: number | null = null;
  private afterId: number | null = null;
  /** 스냅샷 ID를 발급한 문서 epoch — ID는 문서마다 0부터 다시 매겨진다 */
  private epoch: string | null = null;

  static recorded(operationType: string, beforeId: number, afterId: number,
    before: DocumentPosition, after: DocumentPosition): SnapshotCommand {
    const command = new SnapshotCommand(operationType, before, after, null);
    command.beforeId = beforeId;
    command.afterId = afterId;
    return command;
  }

  /**
   * @param operationType 작업 종류 (예: 'pasteInternal', 'deleteControl')
   * @param cursorBefore 작업 전 커서 위치
   * @param operation 실제 작업을 수행하는 함수. 작업 후 커서 위치를 반환.
   */
  constructor(
    operationType: string,
    private cursorBefore: DocumentPosition,
    private cursorAfter: DocumentPosition,
    private operation: ((wasm: WasmBridge) => DocumentPosition) | null,
  ) {
    this.type = `snapshot:${operationType}`;
  }

  execute(wasm: WasmBridge): DocumentPosition {
    if (this.afterId !== null) {
      // Redo: after 스냅샷으로 복원
      try {
        wasm.restoreSnapshot(this.afterId);
      } catch (error: unknown) {
        if (isInvalidEditHandleError(error)) throw asInvalidEditHandleError(error);
        throw error;
      }
      return { ...this.cursorAfter };
    }

    // 최초 실행: before 저장 → 작업 수행 → after 저장
    this.epoch = documentEpochOf(wasm);
    this.beforeId = wasm.saveSnapshot();
    try {
      const operation = this.operation;
      if (operation) this.cursorAfter = withHistorySuspended(wasm, () => operation(wasm));
      this.afterId = wasm.saveSnapshot();
    } catch (error) {
      this.discard(wasm);
      throw error;
    }

    // operation 참조 해제 (클로저에 캡처된 리소스 해제)
    this.operation = null;

    return { ...this.cursorAfter };
  }

  undo(wasm: WasmBridge): DocumentPosition {
    if (this.beforeId !== null) {
      try {
        wasm.restoreSnapshot(this.beforeId);
      } catch (error: unknown) {
        if (isInvalidEditHandleError(error)) throw asInvalidEditHandleError(error);
        throw error;
      }
    }
    return { ...this.cursorBefore };
  }

  recordedLabels(wasm: WasmBridge): { undoLabel: string; redoLabel: string } | null {
    if (this.beforeId === null) return null;
    this.epoch ??= documentEpochOf(wasm);
    const raw = rawGateDoc(wasm)?.snapshotLabels?.(this.beforeId);
    if (!raw) return null;
    return JSON.parse(raw) as { undoLabel: string; redoLabel: string };
  }

  mergeWith(): null { return null; }

  discard(wasm: WasmBridge): void {
    // 재로드된 다른 문서의 같은 번호 스냅샷을 지우지 않는다
    const owned = this.epoch === null || documentEpochOf(wasm) === this.epoch;
    if (this.beforeId !== null) {
      if (owned) wasm.discardSnapshot(this.beforeId);
      this.beforeId = null;
    }
    if (this.afterId !== null) {
      if (owned) wasm.discardSnapshot(this.afterId);
      this.afterId = null;
    }
  }
}

function documentEpochOf(wasm: WasmBridge): string | null {
  return stateLabelOf(wasm)?.split(':')[0] ?? null;
}

/**
 * 여러 개체와 연결선을 함께 바꾸는 한 제스처(다중 선택 드래그, 키보드 이동)를
 * 정확한 히스토리 항목 하나로 기록한다. 첫 실제 변이 직전의 begin()이 제스처
 * 이전 문서와 라벨을 봉인하고, finish()가 이후 스냅샷과 짝지은 SnapshotCommand를
 * 돌려준다 — 선택된 모든 대상과 연결선 갱신이 같은 (이전, 이후) 라벨 쌍에 담긴다.
 */
export class ObjectGestureRecorder {
  private beforeId: number | null = null;

  constructor(
    private readonly operationType: string,
    private readonly cursor: DocumentPosition,
  ) {}

  get started(): boolean { return this.beforeId !== null; }

  /** 첫 실제 변이 직전에 호출한다. 두 번째 호출부터는 아무 일도 하지 않는다. */
  begin(wasm: WasmBridge): void {
    if (this.beforeId === null) this.beforeId = wasm.saveSnapshot();
  }

  /** 마지막 변이 뒤 호출한다. 변이가 없었으면 null. */
  finish(wasm: WasmBridge): EditCommand | null {
    const beforeId = this.beforeId;
    if (beforeId === null) return null;
    this.beforeId = null;
    const afterId = wasm.saveSnapshot();
    return SnapshotCommand.recorded(this.operationType, beforeId, afterId, this.cursor, this.cursor);
  }
}
