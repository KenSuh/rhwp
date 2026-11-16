import type { WasmBridge } from '@/core/wasm-bridge';
import type { DocumentPosition } from '@/core/types';
import type { EditCommand } from './command';
import { InvalidEditHandleError } from '@/core/wasm-bridge';

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

/** 스택 내 모든 명령의 discard()를 호출하여 리소스 해제 */
function discardAll(stack: EditCommand[], wasm: WasmBridge): void {
  for (const cmd of stack) {
    cmd.discard?.(wasm);
  }
}

/** Undo/Redo 히스토리 관리 */
export class CommandHistory {
  private undoStack: EditCommand[] = [];
  private redoStack: EditCommand[] = [];
  private maxSize = 1000;

  /** 명령 실행 + 히스토리 기록. 실행 후 커서 위치 반환 */
  execute(command: EditCommand, wasm: WasmBridge): DocumentPosition {
    this.assertRecordingAllowed(command, wasm);
    const cursorAfter = command.execute(wasm);
    if (wasm.editHistoryDisabledReason !== null) {
      command.discard?.(wasm);
      return cursorAfter;
    }

    // 직전 명령과 병합 시도
    if (this.undoStack.length > 0) {
      const last = this.undoStack[this.undoStack.length - 1];
      const merged = last.mergeWith(command, wasm);
      if (merged) {
        this.undoStack[this.undoStack.length - 1] = merged;
        // redo 스택 정리 (리소스 해제 후 비움)
        discardAll(this.redoStack, wasm);
        this.redoStack = [];
        return cursorAfter;
      }
    }

    this.undoStack.push(command);
    // redo 스택 정리 (새 명령 실행 시 redo 불가)
    discardAll(this.redoStack, wasm);
    this.redoStack = [];

    // 크기 제한 — eviction 시 리소스 해제
    if (this.undoStack.length > this.maxSize) {
      const evicted = this.undoStack.shift();
      evicted?.discard?.(wasm);
    }

    return cursorAfter;
  }

  /** Undo — 성공 시 커서 위치 반환, 스택 비었으면 null */
  undo(wasm: WasmBridge): DocumentPosition | null {
    const command = this.undoStack.at(-1);
    if (!command) return null;

    try {
      const cursorAfter = command.undo(wasm);
      this.undoStack.pop();
      this.redoStack.push(command);
      return cursorAfter;
    } catch (error: unknown) {
      if (error instanceof InvalidEditHandleError) return null;
      throw error;
    }
  }

  /** Redo — 성공 시 커서 위치 반환, 스택 비었으면 null */
  redo(wasm: WasmBridge): DocumentPosition | null {
    const command = this.redoStack.at(-1);
    if (!command) return null;

    try {
      const cursorAfter = command.execute(wasm);
      this.redoStack.pop();
      this.undoStack.push(command);
      return cursorAfter;
    } catch (error: unknown) {
      if (error instanceof InvalidEditHandleError) return null;
      throw error;
    }
  }

  /** execute() 없이 히스토리에만 기록 (IME compositionend용 — 텍스트가 이미 문서에 있는 경우) */
  recordWithoutExecute(command: EditCommand, wasm: WasmBridge): void {
    this.assertRecordingAllowed(command, wasm);
    if (wasm.editHistoryDisabledReason !== null) {
      command.discard?.(wasm);
      return;
    }
    // 직전 명령과 병합 시도
    if (this.undoStack.length > 0) {
      const last = this.undoStack[this.undoStack.length - 1];
      const merged = last.mergeWith(command, wasm);
      if (merged) {
        this.undoStack[this.undoStack.length - 1] = merged;
        discardAll(this.redoStack, wasm);
        this.redoStack = [];
        return;
      }
    }

    this.undoStack.push(command);
    discardAll(this.redoStack, wasm);
    this.redoStack = [];

    if (this.undoStack.length > this.maxSize) {
      const evicted = this.undoStack.shift();
      evicted?.discard?.(wasm);
    }
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

  /** 히스토리 초기화 (문서 로드 시). wasm이 있으면 스냅샷 리소스도 해제. */
  clear(wasm?: WasmBridge): void {
    if (wasm) {
      discardAll(this.undoStack, wasm);
      discardAll(this.redoStack, wasm);
    }
    this.undoStack = [];
    this.redoStack = [];
  }
}
