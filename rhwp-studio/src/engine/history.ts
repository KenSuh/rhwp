import type { WasmBridge } from '@/core/wasm-bridge';
import type { DocumentPosition } from '@/core/types';
import type { EditCommand } from './command';
import { InvalidEditHandleError } from '@/core/wasm-bridge';

interface GateWasmDoc {
  stateLabel?: () => string | undefined;
  previousStateLabel?: () => string | undefined;
  editHandleLabels?: (token: string) => string | undefined;
  snapshotLabels?: (beforeId: number) => string | undefined;
  beginCommandInverse?: (expectedLabel: string, producedLabel: string) => string;
  finishCommandInverse?: (scopeToken: string) => void;
  abortCommandInverse?: (scopeToken: string) => void;
  deleteRangeInCellByPath?: (
    sec: number,
    ppi: number,
    pathJson: string,
    startOffset: number,
    endCellParaIdx: number,
    endOffset: number,
  ) => string;
}

export interface HistoryEntry {
  readonly command: EditCommand;
  readonly undoLabel: string;
  readonly redoLabel: string;
}

export function rawGateDoc(wasm: WasmBridge): GateWasmDoc | null {
  return (wasm as unknown as { doc?: GateWasmDoc | null }).doc ?? null;
}

export function isInvalidEditHandleError(error: unknown): boolean {
  return error instanceof InvalidEditHandleError || String(error).includes('INVALID_EDIT_HANDLE:');
}

export function asInvalidEditHandleError(error: unknown): InvalidEditHandleError {
  return error instanceof InvalidEditHandleError ? error : new InvalidEditHandleError(String(error));
}

export function stateLabelOf(wasm: WasmBridge): string | null {
  const doc = rawGateDoc(wasm);
  if (!doc || typeof doc.stateLabel !== 'function') return null;
  try {
    return doc.stateLabel() ?? null;
  } catch {
    return null;
  }
}

function previousStateLabelOf(wasm: WasmBridge): string | null {
  const doc = rawGateDoc(wasm);
  if (!doc || typeof doc.previousStateLabel !== 'function') return null;
  try {
    return doc.previousStateLabel() ?? null;
  } catch {
    return null;
  }
}

function runGateInverse(
  wasm: WasmBridge,
  expectedLabel: string,
  producedLabel: string,
  operation: () => DocumentPosition,
): DocumentPosition {
  const doc = rawGateDoc(wasm);
  if (!doc || typeof doc.beginCommandInverse !== 'function' || typeof doc.finishCommandInverse !== 'function') {
    throw new InvalidEditHandleError('INVALID_EDIT_HANDLE: document gate unavailable');
  }
  let scopeToken: string;
  try {
    scopeToken = doc.beginCommandInverse(expectedLabel, producedLabel);
  } catch (error: unknown) {
    if (isInvalidEditHandleError(error)) throw asInvalidEditHandleError(error);
    throw error;
  }
  let finished = false;
  try {
    const cursorAfter = operation();
    doc.finishCommandInverse(scopeToken);
    finished = true;
    return cursorAfter;
  } catch (error: unknown) {
    if (!finished) {
      try { doc.abortCommandInverse?.(scopeToken); } catch { /* ignore */ }
    }
    if (isInvalidEditHandleError(error)) throw asInvalidEditHandleError(error);
    throw error;
  }
}

const suspended = new WeakMap<WasmBridge, number>();

export class HistorySuspendedError extends Error {
  constructor() { super('History recording is suspended during a snapshot operation'); }
}

export function withHistorySuspended<T>(wasm: WasmBridge, operation: () => T): T {
  const depth = suspended.get(wasm) ?? 0;
  suspended.set(wasm, depth + 1);
  try { return wasm.withoutEditCapture(operation); }
  finally {
    if (depth === 0) suspended.delete(wasm);
    else suspended.set(wasm, depth);
  }
}

/** 스택 내 모든 엔트리의 discard()를 호출하여 리소스 해제 */
function discardEntries(entries: HistoryEntry[], wasm: WasmBridge): void {
  for (const entry of entries) {
    entry.command.discard?.(wasm);
  }
}

/**
 * 적용 라벨이 live와 같은 가장 높은 항목(꼭대기 포함) 위의 항목을 스택에서 떼어 돌려준다.
 * live가 없거나 맞는 항목이 없으면 스택 전체를 뗀다.
 */
function applicableSuffix(
  stack: HistoryEntry[],
  labelOf: (entry: HistoryEntry) => string,
  live: string | null,
): HistoryEntry[] {
  let keepUntil = -1;
  if (live !== null) {
    for (let i = stack.length - 1; i >= 0; i--) {
      if (labelOf(stack[i]) === live) {
        keepUntil = i;
        break;
      }
    }
  }
  return stack.splice(keepUntil + 1);
}

/** Undo/Redo 히스토리 관리 */
export class CommandHistory {
  private undoStack: HistoryEntry[] = [];
  private redoStack: HistoryEntry[] = [];
  private maxSize = 1000;

  /** 명령 실행 + 히스토리 기록. 실행 후 커서 위치 반환 */
  execute(command: EditCommand, wasm: WasmBridge): DocumentPosition {
    this.assertRecordingAllowed(command, wasm);
    const beforeLabel = stateLabelOf(wasm);
    const cursorAfter = command.execute(wasm);
    const afterLabel = stateLabelOf(wasm);
    if (wasm.editHistoryDisabledReason !== null || beforeLabel === null || afterLabel === null) {
      command.discard?.(wasm);
      return cursorAfter;
    }

    const entry: HistoryEntry = {
      command,
      undoLabel: afterLabel,
      redoLabel: beforeLabel,
    };

    // 직전 명령과 연속 상태 라벨이 일치할 때만 병합 시도
    if (this.undoStack.length > 0) {
      const last = this.undoStack[this.undoStack.length - 1];
      if (last.undoLabel === entry.redoLabel) {
        const merged = last.command.mergeWith(command, wasm);
        if (merged) {
          this.undoStack[this.undoStack.length - 1] = {
            command: merged,
            undoLabel: entry.undoLabel,
            redoLabel: last.redoLabel,
          };
          discardEntries(this.redoStack, wasm);
          this.redoStack = [];
          return cursorAfter;
        }
      }
    }

    this.undoStack.push(entry);
    // redo 스택 정리 (새 명령 실행 시 redo 불가)
    discardEntries(this.redoStack, wasm);
    this.redoStack = [];

    // 크기 제한 — eviction 시 리소스 해제
    if (this.undoStack.length > this.maxSize) {
      const evicted = this.undoStack.shift();
      evicted?.command.discard?.(wasm);
    }

    return cursorAfter;
  }

  /** Undo — 성공 시 커서 위치 반환, 스택 비었거나 거부 시 null */
  undo(wasm: WasmBridge): DocumentPosition | null {
    const entry = this.undoStack.at(-1);
    if (!entry) return null;

    try {
      const cursorAfter = runGateInverse(
        wasm,
        entry.undoLabel,
        entry.redoLabel,
        () => entry.command.undo(wasm),
      );
      this.undoStack.pop();
      this.redoStack.push(entry);
      return cursorAfter;
    } catch (error: unknown) {
      if (error instanceof InvalidEditHandleError) {
        this.reconcileWithLiveLabel(wasm);
        return null;
      }
      throw error;
    }
  }

  /** Redo — 성공 시 커서 위치 반환, 스택 비었거나 거부 시 null */
  redo(wasm: WasmBridge): DocumentPosition | null {
    const entry = this.redoStack.at(-1);
    if (!entry) return null;

    try {
      const cursorAfter = runGateInverse(
        wasm,
        entry.redoLabel,
        entry.undoLabel,
        () => entry.command.execute(wasm),
      );
      this.redoStack.pop();
      this.undoStack.push(entry);
      return cursorAfter;
    } catch (error: unknown) {
      if (error instanceof InvalidEditHandleError) {
        this.reconcileWithLiveLabel(wasm);
        return null;
      }
      throw error;
    }
  }

  /** execute() 없이 히스토리에만 기록 (IME compositionend, 개체 이동 완료 등) */
  recordWithoutExecute(command: EditCommand, wasm: WasmBridge, beforeLabel?: string | null): void {
    this.assertRecordingAllowed(command, wasm);
    if (wasm.editHistoryDisabledReason !== null) {
      command.discard?.(wasm);
      return;
    }
    const labels = this.resolveRecordedLabels(command, wasm, beforeLabel);
    if (!labels) {
      command.discard?.(wasm);
      return;
    }

    const entry: HistoryEntry = {
      command,
      undoLabel: labels.undoLabel,
      redoLabel: labels.redoLabel,
    };

    // 직전 명령과 연속 상태 라벨이 일치할 때만 병합 시도
    if (this.undoStack.length > 0) {
      const last = this.undoStack[this.undoStack.length - 1];
      if (last.undoLabel === entry.redoLabel) {
        const merged = last.command.mergeWith(command, wasm);
        if (merged) {
          this.undoStack[this.undoStack.length - 1] = {
            command: merged,
            undoLabel: entry.undoLabel,
            redoLabel: last.redoLabel,
          };
          discardEntries(this.redoStack, wasm);
          this.redoStack = [];
          return;
        }
      }
    }

    this.undoStack.push(entry);
    discardEntries(this.redoStack, wasm);
    this.redoStack = [];

    if (this.undoStack.length > this.maxSize) {
      const evicted = this.undoStack.shift();
      evicted?.command.discard?.(wasm);
    }
  }

  private resolveRecordedLabels(
    command: EditCommand,
    wasm: WasmBridge,
    beforeLabel?: string | null,
  ): { undoLabel: string; redoLabel: string } | null {
    const intrinsic = command.recordedLabels?.(wasm) ?? null;
    if (intrinsic) return intrinsic;
    const after = stateLabelOf(wasm);
    const before = beforeLabel ?? previousStateLabelOf(wasm);
    if (!after || !before) return null;
    return { undoLabel: after, redoLabel: before };
  }

  /**
   * 거부(InvalidEditHandle) 뒤 두 스택을 모두 살아 있는 라벨에 맞춘다.
   * 각 스택은 적용 라벨(undo는 undoLabel, redo는 redoLabel)이 live와 같은 가장
   * 높은 항목까지만 남긴다. 게이트가 원자적으로 롤백해 라벨이 그대로면 꼭대기가
   * 여전히 맞아 아무것도 버리지 않고, 대역 밖 변이로 라벨이 바뀌었으면 반대쪽
   * 스택에 남은 유령 항목까지 함께 정리한다.
   */
  private reconcileWithLiveLabel(wasm: WasmBridge): void {
    const live = stateLabelOf(wasm);
    discardEntries(applicableSuffix(this.undoStack, entry => entry.undoLabel, live), wasm);
    discardEntries(applicableSuffix(this.redoStack, entry => entry.redoLabel, live), wasm);
  }

  entryLabels(): {
    undo: Array<{ type: string; undoLabel: string; redoLabel: string }>;
    redo: Array<{ type: string; undoLabel: string; redoLabel: string }>;
  } {
    return {
      undo: this.undoStack.map(e => ({ type: e.command.type, undoLabel: e.undoLabel, redoLabel: e.redoLabel })),
      redo: this.redoStack.map(e => ({ type: e.command.type, undoLabel: e.undoLabel, redoLabel: e.redoLabel })),
    };
  }

  canUndo(): boolean { return this.undoStack.length > 0; }
  canRedo(): boolean { return this.redoStack.length > 0; }

  isSuspended(wasm: WasmBridge): boolean { return (suspended.get(wasm) ?? 0) > 0; }

  private assertRecordingAllowed(command: EditCommand, wasm: WasmBridge): void {
    if (this.isSuspended(wasm)) {
      command.discard?.(wasm);
      throw new HistorySuspendedError();
    }
  }

  /** 히스토리 초기화 (문서 로드 또는 비기록 변이 시). wasm이 있으면 리소스도 해제. */
  clear(wasm?: WasmBridge): void {
    if (wasm) {
      discardEntries(this.undoStack, wasm);
      discardEntries(this.redoStack, wasm);
    }
    this.undoStack = [];
    this.redoStack = [];
  }
}
