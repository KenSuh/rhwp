// 표 리사이즈 경계가 병합 셀 내부 클릭을 가로채지 않는지 검증한다.
import assert from 'node:assert/strict';
import test from 'node:test';
import { TableResizeRenderer } from '../src/engine/table-resize-renderer';
import { VirtualScroll } from '../src/view/virtual-scroll';
import type { CellBbox } from '../src/core/types';

// v343 interior-case/geometry.json의 실제 page 3 셀 36개다.
const capturedTable: CellBbox[] = [
  { cellIdx: 0, row: 0, col: 0, rowSpan: 1, colSpan: 1, pageIndex: 3, x: 90.3, y: 132.5, w: 81.7, h: 23.1 },
  { cellIdx: 1, row: 0, col: 1, rowSpan: 1, colSpan: 1, pageIndex: 3, x: 171.9, y: 132.5, w: 43.8, h: 23.1 },
  { cellIdx: 2, row: 0, col: 2, rowSpan: 1, colSpan: 3, pageIndex: 3, x: 215.7, y: 132.5, w: 492.4, h: 23.1 },
  { cellIdx: 3, row: 1, col: 0, rowSpan: 5, colSpan: 1, pageIndex: 3, x: 90.3, y: 155.6, w: 81.7, h: 312 },
  { cellIdx: 4, row: 1, col: 1, rowSpan: 1, colSpan: 1, pageIndex: 3, x: 171.9, y: 155.6, w: 43.8, h: 32.1 },
  { cellIdx: 5, row: 1, col: 2, rowSpan: 1, colSpan: 2, pageIndex: 3, x: 215.7, y: 155.6, w: 370.7, h: 32.1 },
  { cellIdx: 6, row: 1, col: 4, rowSpan: 1, colSpan: 1, pageIndex: 3, x: 586.4, y: 155.6, w: 121.7, h: 32.1 },
  { cellIdx: 7, row: 2, col: 1, rowSpan: 1, colSpan: 1, pageIndex: 3, x: 171.9, y: 187.7, w: 43.8, h: 82.8 },
  { cellIdx: 8, row: 2, col: 2, rowSpan: 1, colSpan: 2, pageIndex: 3, x: 215.7, y: 187.7, w: 370.7, h: 82.8 },
  { cellIdx: 9, row: 2, col: 4, rowSpan: 2, colSpan: 1, pageIndex: 3, x: 586.4, y: 187.7, w: 121.7, h: 146 },
  { cellIdx: 10, row: 3, col: 1, rowSpan: 1, colSpan: 1, pageIndex: 3, x: 171.9, y: 270.5, w: 43.8, h: 63.2 },
  { cellIdx: 11, row: 3, col: 2, rowSpan: 1, colSpan: 2, pageIndex: 3, x: 215.7, y: 270.5, w: 370.7, h: 63.2 },
  { cellIdx: 12, row: 4, col: 1, rowSpan: 1, colSpan: 1, pageIndex: 3, x: 171.9, y: 333.7, w: 43.8, h: 88.7 },
  { cellIdx: 13, row: 4, col: 2, rowSpan: 1, colSpan: 2, pageIndex: 3, x: 215.7, y: 333.7, w: 370.7, h: 88.7 },
  { cellIdx: 14, row: 4, col: 4, rowSpan: 1, colSpan: 1, pageIndex: 3, x: 586.4, y: 333.7, w: 121.7, h: 88.7 },
  { cellIdx: 15, row: 5, col: 1, rowSpan: 1, colSpan: 1, pageIndex: 3, x: 171.9, y: 422.4, w: 43.8, h: 45.3 },
  { cellIdx: 16, row: 5, col: 2, rowSpan: 1, colSpan: 2, pageIndex: 3, x: 215.7, y: 422.4, w: 370.7, h: 45.3 },
  { cellIdx: 17, row: 5, col: 4, rowSpan: 1, colSpan: 1, pageIndex: 3, x: 586.4, y: 422.4, w: 121.7, h: 45.3 },
  { cellIdx: 18, row: 6, col: 0, rowSpan: 1, colSpan: 1, pageIndex: 3, x: 90.3, y: 467.7, w: 81.7, h: 49.6 },
  { cellIdx: 19, row: 6, col: 1, rowSpan: 1, colSpan: 1, pageIndex: 3, x: 171.9, y: 467.7, w: 43.8, h: 49.6 },
  { cellIdx: 20, row: 6, col: 2, rowSpan: 1, colSpan: 2, pageIndex: 3, x: 215.7, y: 467.7, w: 370.7, h: 49.6 },
  { cellIdx: 21, row: 6, col: 4, rowSpan: 1, colSpan: 1, pageIndex: 3, x: 586.4, y: 467.7, w: 121.7, h: 49.6 },
  { cellIdx: 22, row: 7, col: 0, rowSpan: 1, colSpan: 5, pageIndex: 3, x: 90.3, y: 517.2, w: 617.9, h: 51.2 },
  { cellIdx: 23, row: 8, col: 0, rowSpan: 7, colSpan: 1, pageIndex: 3, x: 90.3, y: 568.4, w: 81.7, h: 330.4 },
  { cellIdx: 24, row: 8, col: 1, rowSpan: 1, colSpan: 1, pageIndex: 3, x: 171.9, y: 568.4, w: 43.8, h: 40.8 },
  { cellIdx: 25, row: 8, col: 2, rowSpan: 1, colSpan: 3, pageIndex: 3, x: 215.7, y: 568.4, w: 492.4, h: 40.8 },
  { cellIdx: 26, row: 9, col: 1, rowSpan: 3, colSpan: 1, pageIndex: 3, x: 171.9, y: 609.2, w: 43.8, h: 83.6 },
  { cellIdx: 27, row: 9, col: 2, rowSpan: 3, colSpan: 1, pageIndex: 3, x: 215.7, y: 609.2, w: 39, h: 83.6 },
  { cellIdx: 28, row: 9, col: 3, rowSpan: 1, colSpan: 2, pageIndex: 3, x: 254.7, y: 609.2, w: 453.4, h: 27.8 },
  { cellIdx: 29, row: 10, col: 3, rowSpan: 1, colSpan: 2, pageIndex: 3, x: 254.7, y: 637, w: 453.4, h: 27.8 },
  { cellIdx: 30, row: 11, col: 3, rowSpan: 1, colSpan: 2, pageIndex: 3, x: 254.7, y: 664.8, w: 453.4, h: 28 },
  { cellIdx: 31, row: 12, col: 1, rowSpan: 1, colSpan: 1, pageIndex: 3, x: 171.9, y: 692.8, w: 43.8, h: 40.8 },
  { cellIdx: 32, row: 12, col: 2, rowSpan: 1, colSpan: 3, pageIndex: 3, x: 215.7, y: 692.8, w: 492.4, h: 40.8 },
  { cellIdx: 33, row: 13, col: 1, rowSpan: 2, colSpan: 1, pageIndex: 3, x: 171.9, y: 733.6, w: 43.8, h: 165.2 },
  { cellIdx: 34, row: 13, col: 2, rowSpan: 1, colSpan: 3, pageIndex: 3, x: 215.7, y: 733.6, w: 492.4, h: 61.4 },
  { cellIdx: 35, row: 14, col: 2, rowSpan: 1, colSpan: 3, pageIndex: 3, x: 215.7, y: 795, w: 492.4, h: 103.8 },
];
const renderer = TableResizeRenderer.prototype;
const cell = (overrides: Partial<CellBbox>): CellBbox => ({ cellIdx: 0, row: 0, col: 0, rowSpan: 1, colSpan: 1, pageIndex: 3, x: 0, y: 0, w: 100, h: 50, ...overrides });
const unmerged = [cell({}), cell({ cellIdx: 1, col: 1, x: 100 }), cell({ cellIdx: 2, row: 1, y: 50 }), cell({ cellIdx: 3, row: 1, col: 1, x: 100, y: 50 })];
const splitRow = [cell({}), cell({ row: 1, y: 50 }), cell({ col: 1, x: 100, h: 100, rowSpan: 2 }), cell({ col: 2, x: 200 }), cell({ row: 1, col: 2, x: 200, y: 50 })];
const transpose = (boxes: CellBbox[]) => boxes.map((b) => ({ ...b, x: b.y, y: b.x, w: b.h, h: b.w, row: b.col, col: b.row, rowSpan: b.colSpan, colSpan: b.rowSpan }));

test('captured merged-cell interior does not start a border resize', () => {
  const hit = renderer.hitTestBorder(625.19375, 270.5, capturedTable);
  assert.equal(hit, null);
});

for (const [name, x, y, type, index] of [
  ['captured real row', 300, 270.5, 'row', 3],
  ['captured real column', 586.4, 230, 'col', 5],
] as const) {
  test(name + ' retains the coordinate index used by resize callers', () => {
    const hit = renderer.hitTestBorder(x, y, capturedTable);
    assert.deepEqual(hit, { type, index, pageIndex: 3 });
  });
}

test('rowSpan leaves a hole between disjoint real row segments', () => {
  const hit = renderer.hitTestBorder(150, 50, splitRow);
  assert.equal(hit, null);
});

test('colSpan leaves a hole between disjoint real column segments', () => {
  const hit = renderer.hitTestBorder(50, 150, transpose(splitRow));
  assert.equal(hit, null);
});

test('shared and adjacent cell edges merge without closing a merged-cell hole', () => {
  const lines = renderer.computeBorderLines(splitRow).rowLines.filter((line) => line.y === 50);
  assert.deepEqual(lines, [{ y: 50, xStart: 0, xEnd: 100, index: 1 }, { y: 50, xStart: 200, xEnd: 300, index: 1 }]);
});

test('every captured coordinate index still resolves to its original rounded coordinate', () => {
  const rounded = (value: number) => Math.round(value * 10) / 10;
  const ys = [...new Set(capturedTable.flatMap((b) => [rounded(b.y), rounded(b.y + b.h)]))].sort((a, b) => a - b);
  const xs = [...new Set(capturedTable.flatMap((b) => [rounded(b.x), rounded(b.x + b.w)]))].sort((a, b) => a - b);
  const { rowLines, colLines } = renderer.computeBorderLines(capturedTable);
  assert.deepEqual(ys.map((_, index) => rowLines.find((line) => line.index === index)?.y), ys);
  assert.deepEqual(xs.map((_, index) => colLines.find((line) => line.index === index)?.x), xs);
});

for (const [x, expected] of [[104, { type: 'row', index: 1, pageIndex: 3 }], [104.01, null]] as const) {
  test('real segment endpoint tolerance at x=' + x, () => {
    const hit = renderer.hitTestBorder(x, 50, splitRow);
    assert.deepEqual(hit, expected);
  });
}

for (const [name, x, y, expected] of [
  ['unmerged row', 50, 50, { type: 'row', index: 1, pageIndex: 3 }],
  ['unmerged column', 100, 25, { type: 'col', index: 1, pageIndex: 3 }],
  ['row priority at a corner', 100, 50, { type: 'row', index: 1, pageIndex: 3 }],
  ['inclusive row tolerance', 50, -4, { type: 'row', index: 0, pageIndex: 3 }],
  ['outside row tolerance', 50, -4.01, null],
  ['inclusive column tolerance', 204, 25, { type: 'col', index: 2, pageIndex: 3 }],
  ['outside column tolerance', 204.01, 25, null],
] as const) {
  test(name, () => {
    const hit = renderer.hitTestBorder(x, y, unmerged);
    assert.deepEqual(hit, expected);
  });
}

test('empty input has no border', () => {
  const hit = renderer.hitTestBorder(0, 0, []);
  assert.equal(hit, null);
});

test('another page cannot extend the queried page border', () => {
  const hit = renderer.hitTestBorder(500, 50, [...unmerged, cell({ pageIndex: 4, x: 1000 })]);
  assert.equal(hit, null);
});

// 브라우저 없는 unit test에서 사용하는 DOM 연산만 대체한다. 실제 UI QA는 별도다.
class MarkerElement {
  id = '';
  className = '';
  clientWidth = 1000;
  style = { cssText: '' };
  parentElement: MarkerElement | null = null;
  children: MarkerElement[] = [];
  appendChild(child: MarkerElement) { child.parentElement = this; this.children.push(child); return child; }
  querySelector(selector: string) { return this.children.find((child) => selector === '#' + child.id || selector === '.' + child.className) ?? null; }
  getAttribute(name: string) { return name === 'style' ? this.style.cssText : null; }
  remove() {
    if (this.parentElement) this.parentElement.children = this.parentElement.children.filter((child) => child !== this);
    this.parentElement = null;
  }
}

function withMarker(run: (instance: TableResizeRenderer, layer: HTMLDivElement) => void) {
  const original = Object.getOwnPropertyDescriptor(globalThis, 'document');
  Object.defineProperty(globalThis, 'document', { configurable: true, value: { createElement: () => new MarkerElement() } });
  try {
    const container = document.createElement('div');
    const content = document.createElement('div');
    content.id = 'scroll-content';
    container.appendChild(content);
    const instance = new TableResizeRenderer(container, new VirtualScroll());
    const layer = content.querySelector<HTMLDivElement>('.table-resize-layer');
    assert.ok(layer);
    run(instance, layer);
  } finally {
    if (original) Object.defineProperty(globalThis, 'document', original);
    else Reflect.deleteProperty(globalThis, 'document');
  }
}

test('row hover marker paints both real segments and leaves the merged cell clear', () => {
  withMarker((instance, layer) => {
    instance.showMarker({ type: 'row', index: 1, pageIndex: 3 }, splitRow, 2);
    const styles = Array.from(layer.children, (element) => element.getAttribute('style'));
    assert.deepEqual(styles, [
      'position:absolute;left:500px;top:98.5px;width:200px;height:3px;background:rgba(0, 120, 215, 0.5);pointer-events:none;',
      'position:absolute;left:900px;top:98.5px;width:200px;height:3px;background:rgba(0, 120, 215, 0.5);pointer-events:none;',
    ]);
  });
});

test('column hover marker paints both real segments', () => {
  withMarker((instance, layer) => {
    instance.showMarker({ type: 'col', index: 1, pageIndex: 3 }, transpose(splitRow), 2);
    const styles = Array.from(layer.children, (element) => element.getAttribute('style'));
    assert.deepEqual(styles, [
      'position:absolute;left:598.5px;top:0px;width:3px;height:200px;background:rgba(0, 120, 215, 0.5);pointer-events:none;',
      'position:absolute;left:598.5px;top:400px;width:3px;height:200px;background:rgba(0, 120, 215, 0.5);pointer-events:none;',
    ]);
  });
});

test('clear removes every marker segment', () => {
  withMarker((instance, layer) => {
    instance.showMarker({ type: 'row', index: 1, pageIndex: 3 }, splitRow, 1);
    instance.clear();
    assert.equal(layer.children.length, 0);
  });
});

test('hover marker ignores a different page with the same coordinate index', () => {
  withMarker((instance, layer) => {
    instance.showMarker({ type: 'row', index: 0, pageIndex: 4 }, unmerged, 1);
    assert.equal(layer.children.length, 0);
  });
});

test('row drag guide replaces hover segments and uses only the selected page', () => {
  withMarker((instance, layer) => {
    instance.showMarker({ type: 'row', index: 1, pageIndex: 3 }, splitRow, 2);
    instance.showDragMarker('row', 60, 3, [...splitRow, cell({ pageIndex: 4, x: 2000 })], 2);
    assert.deepEqual(Array.from(layer.children, (element) => element.getAttribute('style')), [
      'position:absolute;left:500px;top:118.5px;width:600px;height:3px;background:rgba(0, 120, 215, 0.5);pointer-events:none;',
    ]);
  });
});

test('column drag guide replaces hover segments and uses only the selected page', () => {
  withMarker((instance, layer) => {
    instance.showMarker({ type: 'col', index: 1, pageIndex: 3 }, transpose(splitRow), 2);
    instance.showDragMarker('col', 60, 3, [...transpose(splitRow), cell({ pageIndex: 4, y: 2000 })], 2);
    assert.deepEqual(Array.from(layer.children, (element) => element.getAttribute('style')), [
      'position:absolute;left:618.5px;top:0px;width:3px;height:600px;background:rgba(0, 120, 215, 0.5);pointer-events:none;',
    ]);
  });
});
