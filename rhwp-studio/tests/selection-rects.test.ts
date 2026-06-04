// 선택 영역 사각형 병합 회귀 테스트.
import assert from 'node:assert/strict';
import test from 'node:test';
import {
  clipSelectionRectToPage,
  mergeSelectionRects,
  selectionRectsFromBodyTextRuns,
  selectionRectsFromCellTextRuns,
} from '../src/engine/selection-rects';

test('mergeSelectionRects merges fragmented same-line highlights', () => {
  const merged = mergeSelectionRects([
    { pageIndex: 0, x: 10, y: 20, width: 35, height: 18 },
    { pageIndex: 0, x: 47, y: 21, width: 28, height: 17 },
    { pageIndex: 0, x: 77, y: 20, width: 24, height: 18 },
  ]);

  assert.equal(merged.length, 1);
  assert.deepEqual(merged[0], {
    pageIndex: 0,
    x: 10,
    y: 20,
    width: 91,
    height: 18,
  });
});

test('mergeSelectionRects keeps different visual lines separate', () => {
  const merged = mergeSelectionRects([
    { pageIndex: 0, x: 10, y: 20, width: 35, height: 18 },
    { pageIndex: 0, x: 12, y: 46, width: 40, height: 18 },
  ]);

  assert.equal(merged.length, 2);
});

test('mergeSelectionRects keeps different pages separate', () => {
  const merged = mergeSelectionRects([
    { pageIndex: 0, x: 10, y: 20, width: 35, height: 18 },
    { pageIndex: 1, x: 47, y: 20, width: 35, height: 18 },
  ]);

  assert.equal(merged.length, 2);
});

test('clipSelectionRectToPage clips highlight overflow to the page width', () => {
  const clipped = clipSelectionRectToPage(
    { pageIndex: 0, x: 90, y: 20, width: 40, height: 18 },
    100,
  );

  assert.deepEqual(clipped, {
    pageIndex: 0,
    x: 90,
    y: 20,
    width: 10,
    height: 18,
  });
});

test('clipSelectionRectToPage drops highlights completely outside the page', () => {
  const clipped = clipSelectionRectToPage(
    { pageIndex: 0, x: 110, y: 20, width: 40, height: 18 },
    100,
  );

  assert.equal(clipped, null);
});

test('selectionRectsFromCellTextRuns includes wrapped visual lines in the same cell paragraph', () => {
  const rects = selectionRectsFromCellTextRuns(
    0,
    [
      {
        text: '첫번째줄',
        x: 10,
        y: 20,
        w: 80,
        h: 18,
        charX: [0, 20, 40, 60, 80],
        secIdx: 0,
        parentParaIdx: 3,
        controlIdx: 0,
        cellIdx: 12,
        cellParaIdx: 4,
        charStart: 0,
      },
      {
        text: '자동줄바꿈',
        x: 30,
        y: 42,
        w: 100,
        h: 18,
        charX: [0, 20, 40, 60, 80, 100],
        secIdx: 0,
        parentParaIdx: 3,
        controlIdx: 0,
        cellIdx: 12,
        cellParaIdx: 4,
        charStart: 4,
      },
    ],
    {
      sectionIndex: 0,
      parentParaIndex: 3,
      controlIndex: 0,
      cellIndex: 12,
      startCellParaIndex: 4,
      startOffset: 0,
      endCellParaIndex: 4,
      endOffset: 9,
    },
  );

  assert.deepEqual(rects, [
    { pageIndex: 0, x: 10, y: 20, width: 80, height: 18 },
    { pageIndex: 0, x: 30, y: 42, width: 100, height: 18 },
  ]);
});

test('selectionRectsFromBodyTextRuns includes wrapped visual lines in the same body paragraph', () => {
  const rects = selectionRectsFromBodyTextRuns(
    0,
    [
      {
        text: '본문첫줄',
        x: 20,
        y: 30,
        w: 80,
        h: 18,
        charX: [0, 20, 40, 60, 80],
        secIdx: 0,
        paraIdx: 7,
        charStart: 0,
      },
      {
        text: '자동줄바꿈',
        x: 20,
        y: 52,
        w: 100,
        h: 18,
        charX: [0, 20, 40, 60, 80, 100],
        secIdx: 0,
        paraIdx: 7,
        charStart: 4,
      },
    ],
    {
      sectionIndex: 0,
      startParagraphIndex: 7,
      startOffset: 0,
      endParagraphIndex: 7,
      endOffset: 9,
    },
  );

  assert.deepEqual(rects, [
    { pageIndex: 0, x: 20, y: 30, width: 80, height: 18 },
    { pageIndex: 0, x: 20, y: 52, width: 100, height: 18 },
  ]);
});

test('selectionRectsFromCellTextRuns matches the target nested cell path', () => {
  const rects = selectionRectsFromCellTextRuns(
    0,
    [
      {
        text: '내부셀',
        x: 10,
        y: 20,
        w: 60,
        h: 18,
        charX: [0, 20, 40, 60],
        secIdx: 0,
        parentParaIdx: 3,
        controlIdx: 0,
        cellIdx: 1,
        cellParaIdx: 0,
        charStart: 0,
        cellPath: [
          { controlIndex: 0, cellIndex: 1, cellParaIndex: 0 },
          { controlIndex: 2, cellIndex: 3, cellParaIndex: 5 },
        ],
      },
      {
        text: '다른셀',
        x: 80,
        y: 20,
        w: 60,
        h: 18,
        charX: [0, 20, 40, 60],
        secIdx: 0,
        parentParaIdx: 3,
        controlIdx: 0,
        cellIdx: 1,
        cellParaIdx: 0,
        charStart: 0,
        cellPath: [
          { controlIndex: 0, cellIndex: 1, cellParaIndex: 0 },
          { controlIndex: 2, cellIndex: 4, cellParaIndex: 5 },
        ],
      },
    ],
    {
      sectionIndex: 0,
      parentParaIndex: 3,
      controlIndex: 0,
      cellIndex: 1,
      cellPath: [
        { controlIndex: 0, cellIndex: 1, cellParaIndex: 0 },
        { controlIndex: 2, cellIndex: 3, cellParaIndex: 5 },
      ],
      startCellParaIndex: 5,
      startOffset: 0,
      endCellParaIndex: 5,
      endOffset: 3,
    },
  );

  assert.deepEqual(rects, [
    { pageIndex: 0, x: 10, y: 20, width: 60, height: 18 },
  ]);
});
