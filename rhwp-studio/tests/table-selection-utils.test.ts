// rhwp식 표 셀 선택 범위 판정의 병합 셀/부분 선택 회귀를 고정한다.
import assert from 'node:assert/strict';
import test from 'node:test';
import { isFullRowCellSelectionCoverage } from '../src/engine/table-selection-utils';

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
