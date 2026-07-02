// 선택 셀 삭제/오려두기의 전체 행/부분 선택 분기 계약을 검증한다.
import assert from 'node:assert/strict';
import test from 'node:test';
import { resolveCellCutDeletePlan } from '../src/engine/table-cell-delete-policy';

test('partial cell selections clear contents without prompting', () => {
  assert.equal(
    resolveCellCutDeletePlan({ isFullRowSelection: false }),
    'clear-contents',
  );
});

test('full-row cell selections request the rhwp keep-shape/delete-cells choice first', () => {
  assert.equal(
    resolveCellCutDeletePlan({ isFullRowSelection: true }),
    'request-choice',
  );
});

test('full-row keep-shape choice clears only contents', () => {
  assert.equal(
    resolveCellCutDeletePlan({ isFullRowSelection: true, choice: 'keep-shape' }),
    'clear-contents',
  );
});

test('full-row delete-cells choice deletes selected rows', () => {
  assert.equal(
    resolveCellCutDeletePlan({ isFullRowSelection: true, choice: 'delete-cells' }),
    'delete-cells',
  );
});

test('full-row cancel choice aborts cut/delete', () => {
  assert.equal(
    resolveCellCutDeletePlan({ isFullRowSelection: true, choice: 'cancel' }),
    'cancel',
  );
});
