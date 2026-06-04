// 표 셀 선택 범위 판정을 rhwp식 조작 명령과 테스트에서 공유한다.
import type { CellBbox } from '@/core/types';

export type CellSelectionRange = {
  startRow: number;
  startCol: number;
  endRow: number;
  endCol: number;
};

export type FullRowCellSelectionInput = {
  colCount?: number;
  range: CellSelectionRange;
  excluded?: ReadonlySet<string>;
  bboxes: Pick<CellBbox, 'row' | 'col' | 'rowSpan' | 'colSpan'>[];
};

/**
 * 선택 범위가 각 행의 모든 논리 열을 덮는지 확인한다.
 *
 * 병합 셀은 rowSpan/colSpan으로 여러 논리 칸을 차지하므로, 단순히
 * startCol/endCol만 보는 대신 실제 선택 bbox가 행별 열 전체를 덮는지 계산한다.
 */
export function isFullRowCellSelectionCoverage(selection: FullRowCellSelectionInput): boolean {
  const colCount = selection.colCount;
  if (typeof colCount !== 'number' || colCount <= 0 || selection.excluded?.size) {
    return false;
  }
  if (selection.range.startCol > 0 || selection.range.endCol < colCount - 1) {
    return false;
  }

  for (let row = selection.range.startRow; row <= selection.range.endRow; row += 1) {
    const covered = new Array<boolean>(colCount).fill(false);
    for (const bbox of selection.bboxes) {
      const rowEnd = bbox.row + Math.max(1, bbox.rowSpan) - 1;
      if (row < bbox.row || row > rowEnd) continue;
      const colStart = Math.max(0, bbox.col);
      const colEnd = Math.min(colCount - 1, bbox.col + Math.max(1, bbox.colSpan) - 1);
      for (let col = colStart; col <= colEnd; col += 1) {
        covered[col] = true;
      }
    }
    if (!covered.every(Boolean)) return false;
  }
  return true;
}
