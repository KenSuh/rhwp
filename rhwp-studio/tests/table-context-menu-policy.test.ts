// 표 컨텍스트 메뉴의 rhwp parity 필수 항목을 고정한다.
import assert from 'node:assert/strict';
import test from 'node:test';
import { buildTableContextMenuItems } from '../src/engine/table-context-menu-policy';

function commandIds(inCellSelection: boolean, hasSelection = false): string[] {
  return buildTableContextMenuItems({ inCellSelection, hasSelection })
    .filter((item) => item.type === 'command')
    .map((item) => item.commandId ?? '');
}

test('cell selection context menu exposes equal height and width commands', () => {
  const ids = commandIds(true);
  const heightIndex = ids.indexOf('table:cell-height-equal');

  assert.ok(heightIndex >= 0, 'cell selection menu must include equal-height command');
  assert.deepEqual(ids.slice(heightIndex, heightIndex + 4), [
    'table:cell-height-equal',
    'table:cell-width-equal',
    'table:cell-merge',
    'table:cell-split',
  ]);
});

test('table text context menu keeps cell-selection-only sizing commands hidden', () => {
  const ids = commandIds(false, true);

  assert.equal(ids.includes('table:cell-height-equal'), false);
  assert.equal(ids.includes('table:cell-width-equal'), false);
  assert.equal(ids.includes('edit:delete'), true);
});
