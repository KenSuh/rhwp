// rhwp식 표 셀 선택 범위 판정의 병합 셀/부분 선택 회귀를 고정한다.
import assert from 'node:assert/strict';
import test from 'node:test';
import {
  computeCellDragRange,
  isFullRowCellSelectionCoverage,
} from '../src/engine/table-selection-utils';

test('full-row cell selection covers every logical column', () => {
  assert.equal(
    isFullRowCellSelectionCoverage({
      colCount: 3,
      range: { startRow: 0, startCol: 0, endRow: 0, endCol: 2 },
      bboxes: [
        { row: 0, col: 0, rowSpan: 1, colSpan: 1 },
        { row: 0, col: 1, rowSpan: 1, colSpan: 1 },
        { row: 0, col: 2, rowSpan: 1, colSpan: 1 },
      ],
    }),
    true,
  );
});

test('partial cell selection is not treated as a full-row delete candidate', () => {
  assert.equal(
    isFullRowCellSelectionCoverage({
      colCount: 3,
      range: { startRow: 0, startCol: 1, endRow: 0, endCol: 2 },
      bboxes: [
        { row: 0, col: 1, rowSpan: 1, colSpan: 1 },
        { row: 0, col: 2, rowSpan: 1, colSpan: 1 },
      ],
    }),
    false,
  );
});

test('merged cells count toward the full-row coverage they occupy', () => {
  assert.equal(
    isFullRowCellSelectionCoverage({
      colCount: 4,
      range: { startRow: 0, startCol: 0, endRow: 1, endCol: 3 },
      bboxes: [
        { row: 0, col: 0, rowSpan: 2, colSpan: 2 },
        { row: 0, col: 2, rowSpan: 1, colSpan: 2 },
        { row: 1, col: 2, rowSpan: 1, colSpan: 1 },
        { row: 1, col: 3, rowSpan: 1, colSpan: 1 },
      ],
    }),
    true,
  );
});

test('holes in merged-cell coverage keep the action in content-clear mode', () => {
  assert.equal(
    isFullRowCellSelectionCoverage({
      colCount: 4,
      range: { startRow: 0, startCol: 0, endRow: 1, endCol: 3 },
      bboxes: [
        { row: 0, col: 0, rowSpan: 2, colSpan: 2 },
        { row: 0, col: 2, rowSpan: 1, colSpan: 2 },
        { row: 1, col: 3, rowSpan: 1, colSpan: 1 },
      ],
    }),
    false,
  );
});

test('excluded cells prevent destructive full-row handling', () => {
  assert.equal(
    isFullRowCellSelectionCoverage({
      colCount: 2,
      range: { startRow: 0, startCol: 0, endRow: 0, endCol: 1 },
      excluded: new Set(['0:1']),
      bboxes: [
        { row: 0, col: 0, rowSpan: 1, colSpan: 1 },
        { row: 0, col: 1, rowSpan: 1, colSpan: 1 },
      ],
    }),
    false,
  );
});

// computeCellDragRange — TBL-SEL-003/004 라이브 범위 add/cancel 회귀 고정.
// 기대값은 /sample.hwpx GearUp editor 실 드래그(synthetic mouse) 관측과 동일하다.
const anchorOf = (row: number, col: number) => ({ startRow: row, startCol: col, endRow: row, endCol: col });

test('drag range grows away from the anchor', () => {
  const anchor = anchorOf(0, 0);
  assert.deepEqual(computeCellDragRange(anchor, { row: 2, col: 0 }), { startRow: 0, startCol: 0, endRow: 2, endCol: 0 });
  assert.deepEqual(computeCellDragRange(anchor, { row: 4, col: 0 }), { startRow: 0, startCol: 0, endRow: 4, endCol: 0 });
});

test('drag range shrinks back toward the anchor (passed cells are not sticky)', () => {
  const anchor = anchorOf(0, 0);
  // 매 이동은 직전 범위가 아니라 anchor 기준으로 재계산되므로 4→2로 줄면 셀이 빠진다.
  assert.deepEqual(computeCellDragRange(anchor, { row: 2, col: 0 }), { startRow: 0, startCol: 0, endRow: 2, endCol: 0 });
  assert.deepEqual(computeCellDragRange(anchor, { row: 0, col: 0 }), { startRow: 0, startCol: 0, endRow: 0, endCol: 0 });
});

test('drag range flips across the anchor', () => {
  const anchor = anchorOf(2, 2);
  assert.deepEqual(computeCellDragRange(anchor, { row: 0, col: 2 }), { startRow: 0, startCol: 2, endRow: 2, endCol: 2 });
  // anchor를 가로질러 반대쪽으로 가면 이전 위쪽 셀이 빠지고 아래쪽으로 뒤집힌다.
  assert.deepEqual(computeCellDragRange(anchor, { row: 4, col: 2 }), { startRow: 2, startCol: 2, endRow: 4, endCol: 2 });
});

test('drag range expands by a merged focus cell rowSpan/colSpan', () => {
  const anchor = anchorOf(0, 0);
  // 병합 셀(0,11 rowSpan2 colSpan8)을 포함하면 끝 행/열이 1,18까지 확장된다.
  assert.deepEqual(
    computeCellDragRange(anchor, { row: 0, col: 11, rowSpan: 2, colSpan: 8 }),
    { startRow: 0, startCol: 0, endRow: 1, endCol: 18 },
  );
});

test('drag range collapses to a single cell back at the anchor', () => {
  const anchor = anchorOf(3, 5);
  assert.deepEqual(computeCellDragRange(anchor, { row: 3, col: 5 }), { startRow: 3, startCol: 5, endRow: 3, endCol: 5 });
});

test('drag range grows horizontally and is symmetric with anchor span', () => {
  // anchor 자체가 병합 셀(rowSpan2 colSpan2)인 경우에도 anchor 전체가 항상 포함된다.
  const anchor = { startRow: 0, startCol: 0, endRow: 1, endCol: 1 };
  assert.deepEqual(computeCellDragRange(anchor, { row: 0, col: 3 }), { startRow: 0, startCol: 0, endRow: 1, endCol: 3 });
  assert.deepEqual(computeCellDragRange(anchor, { row: 3, col: 0 }), { startRow: 0, startCol: 0, endRow: 3, endCol: 1 });
});
