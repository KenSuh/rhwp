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

export type CellDragFocus = {
  row: number;
  col: number;
  rowSpan?: number;
  colSpan?: number;
};

/**
 * 드래그 중 셀 선택 범위를 anchor 셀과 현재 포커스 셀을 모두 감싸는 bbox로 계산한다.
 *
 * rhwp parity (TBL-SEL-003/004): 선택 범위는 mousedown anchor 셀과 현재 마우스가
 * 올라간 셀을 모두 포함하는 최소 사각형이다. 매 이동마다 anchor 기준으로 다시
 * 계산하므로 마우스가 멀어지면 셀이 더해지고 가까워지면 빠진다. 이전에 지나간 셀이
 * 고정 선택으로 남지 않고, anchor를 가로질러 반대 방향으로 가면 자연스럽게 뒤집힌다.
 * 병합 셀은 rowSpan/colSpan만큼 끝 행/열을 차지한다.
 *
 * 순수 함수로 분리해 hitTest/렌더러와 독립적으로 회귀 테스트가 가능하게 한다.
 */
export function computeCellDragRange(
  anchor: CellSelectionRange,
  focus: CellDragFocus,
): CellSelectionRange {
  const focusEndRow = focus.row + Math.max(1, focus.rowSpan ?? 1) - 1;
  const focusEndCol = focus.col + Math.max(1, focus.colSpan ?? 1) - 1;
  return {
    startRow: Math.min(anchor.startRow, focus.row),
    startCol: Math.min(anchor.startCol, focus.col),
    endRow: Math.max(anchor.endRow, focusEndRow),
    endCol: Math.max(anchor.endCol, focusEndCol),
  };
}
