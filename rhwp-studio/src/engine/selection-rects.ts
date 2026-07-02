// 선택 영역 사각형 보정 유틸리티.
import type { CellPathEntry, SelectionRect, TextLayoutRun } from '@/core/types';

export interface CellTextSelectionRange {
  sectionIndex: number;
  parentParaIndex: number;
  controlIndex: number;
  cellIndex: number;
  cellPath?: CellPathEntry[];
  startCellParaIndex: number;
  startOffset: number;
  endCellParaIndex: number;
  endOffset: number;
}

export interface BodyTextSelectionRange {
  sectionIndex: number;
  startParagraphIndex: number;
  startOffset: number;
  endParagraphIndex: number;
  endOffset: number;
}

export function mergeSelectionRects(rects: SelectionRect[]): SelectionRect[] {
  const sorted = [...rects]
    .filter((rect) => rect.width > 0 && rect.height > 0)
    .sort((a, b) => {
      if (a.pageIndex !== b.pageIndex) return a.pageIndex - b.pageIndex;
      if (isSameVisualLine(a, b)) return a.x - b.x;
      return a.y - b.y || a.x - b.x;
    });
  const merged: SelectionRect[] = [];

  for (const rect of sorted) {
    const last = merged[merged.length - 1];
    if (last && isSameSelectionLine(last, rect)) {
      const left = Math.min(last.x, rect.x);
      const top = Math.min(last.y, rect.y);
      const right = Math.max(last.x + last.width, rect.x + rect.width);
      const bottom = Math.max(last.y + last.height, rect.y + rect.height);
      last.x = left;
      last.y = top;
      last.width = right - left;
      last.height = bottom - top;
      continue;
    }
    merged.push({ ...rect });
  }

  return merged;
}

export function clipSelectionRectToPage(rect: SelectionRect, pageWidth: number): SelectionRect | null {
  if (rect.width <= 0 || rect.height <= 0) return null;
  if (!Number.isFinite(pageWidth) || pageWidth <= 0) return { ...rect };

  const left = Math.max(0, rect.x);
  const right = Math.min(pageWidth, rect.x + rect.width);
  if (right <= left) return null;

  return {
    ...rect,
    x: left,
    width: right - left,
  };
}

export function selectionRectsFromCellTextRuns(
  pageIndex: number,
  runs: TextLayoutRun[],
  range: CellTextSelectionRange,
): SelectionRect[] {
  const rects: SelectionRect[] = [];

  for (const run of runs) {
    if (!isRunInCellRange(run, range)) continue;

    const cellParaIndex = getRunCellParaIndex(run, range);
    const charStart = run.charStart;
    if (cellParaIndex === undefined || charStart === undefined) continue;

    const textLength = Array.from(run.text).length;
    if (textLength <= 0) continue;

    const selectedStart = cellParaIndex === range.startCellParaIndex ? range.startOffset : 0;
    const selectedEnd = cellParaIndex === range.endCellParaIndex ? range.endOffset : Number.MAX_SAFE_INTEGER;
    const localStart = Math.max(selectedStart, charStart) - charStart;
    const localEnd = Math.min(selectedEnd, charStart + textLength) - charStart;
    if (localEnd <= localStart) continue;

    const left = run.x + runOffsetToX(run, localStart, textLength);
    const right = run.x + runOffsetToX(run, localEnd, textLength);
    const x = Math.min(left, right);
    const width = Math.abs(right - left);
    if (width <= 0.01 || run.h <= 0) continue;

    rects.push({
      pageIndex,
      x,
      y: run.y,
      width,
      height: run.h,
    });
  }

  return rects;
}

export function selectionRectsFromBodyTextRuns(
  pageIndex: number,
  runs: TextLayoutRun[],
  range: BodyTextSelectionRange,
): SelectionRect[] {
  const rects: SelectionRect[] = [];

  for (const run of runs) {
    if (!isRunInBodyRange(run, range)) continue;

    const paraIndex = run.paraIdx;
    const charStart = run.charStart;
    if (paraIndex === undefined || charStart === undefined) continue;

    const textLength = Array.from(run.text).length;
    if (textLength <= 0) continue;

    const selectedStart = paraIndex === range.startParagraphIndex ? range.startOffset : 0;
    const selectedEnd = paraIndex === range.endParagraphIndex ? range.endOffset : Number.MAX_SAFE_INTEGER;
    const localStart = Math.max(selectedStart, charStart) - charStart;
    const localEnd = Math.min(selectedEnd, charStart + textLength) - charStart;
    if (localEnd <= localStart) continue;

    const left = run.x + runOffsetToX(run, localStart, textLength);
    const right = run.x + runOffsetToX(run, localEnd, textLength);
    const x = Math.min(left, right);
    const width = Math.abs(right - left);
    if (width <= 0.01 || run.h <= 0) continue;

    rects.push({
      pageIndex,
      x,
      y: run.y,
      width,
      height: run.h,
    });
  }

  return rects;
}

function isRunInCellRange(run: TextLayoutRun, range: CellTextSelectionRange): boolean {
  if (
    run.secIdx !== range.sectionIndex ||
    run.parentParaIdx !== range.parentParaIndex
  ) {
    return false;
  }

  if (range.cellPath?.length) {
    if (!run.cellPath || !isSameCellPathTarget(run.cellPath, range.cellPath)) {
      return false;
    }
  } else if (
    run.controlIdx !== range.controlIndex ||
    run.cellIdx !== range.cellIndex
  ) {
    return false;
  }

  const cellParaIndex = getRunCellParaIndex(run, range);
  return cellParaIndex !== undefined &&
    cellParaIndex >= range.startCellParaIndex &&
    cellParaIndex <= range.endCellParaIndex;
}

function isRunInBodyRange(run: TextLayoutRun, range: BodyTextSelectionRange): boolean {
  if (
    run.secIdx !== range.sectionIndex ||
    run.paraIdx === undefined ||
    run.cellPath?.length ||
    run.parentParaIdx !== undefined
  ) {
    return false;
  }

  return run.paraIdx >= range.startParagraphIndex && run.paraIdx <= range.endParagraphIndex;
}

function getRunCellParaIndex(run: TextLayoutRun, range: CellTextSelectionRange): number | undefined {
  const depth = range.cellPath?.length ?? 0;
  if (depth > 0 && run.cellPath?.length === depth) {
    return run.cellPath[depth - 1]?.cellParaIndex;
  }
  return run.cellParaIdx;
}

function isSameCellPathTarget(runPath: CellPathEntry[], rangePath: CellPathEntry[]): boolean {
  if (runPath.length !== rangePath.length) return false;
  return rangePath.every((entry, index) => {
    const runEntry = runPath[index];
    return runEntry?.controlIndex === entry.controlIndex &&
      runEntry.cellIndex === entry.cellIndex;
  });
}

function runOffsetToX(run: TextLayoutRun, localOffset: number, textLength: number): number {
  if (localOffset <= 0) return 0;
  if (localOffset >= textLength) return run.w;

  const position = run.charX[localOffset];
  if (Number.isFinite(position)) return position;
  const fallback = run.charX[run.charX.length - 1];
  return Number.isFinite(fallback) ? fallback : run.w;
}

function isSameSelectionLine(a: SelectionRect, b: SelectionRect): boolean {
  if (!isSameVisualLine(a, b)) return false;

  const gap = b.x - (a.x + a.width);
  const maxInlineGap = Math.max(10, Math.max(a.height, b.height) * 1.2);
  return gap <= maxInlineGap;
}

function isSameVisualLine(a: SelectionRect, b: SelectionRect): boolean {
  if (a.pageIndex !== b.pageIndex) return false;

  const aBottom = a.y + a.height;
  const bBottom = b.y + b.height;
  const verticalOverlap = Math.min(aBottom, bBottom) - Math.max(a.y, b.y);
  const minHeight = Math.min(a.height, b.height);
  const baselineDelta = Math.abs((a.y + a.height / 2) - (b.y + b.height / 2));
  if (verticalOverlap < minHeight * 0.45 && baselineDelta > minHeight * 0.55) return false;
  return true;
}
