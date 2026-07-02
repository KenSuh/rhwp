// 그리드(다중 열) 레이아웃 페이지 히트테스트(getPageAtXY) 회귀 테스트.
import assert from 'node:assert/strict';
import test from 'node:test';
import { VirtualScroll } from '../src/view/virtual-scroll';
import type { PageInfo } from '../src/core/types';

/** 테스트용 페이지 목록 생성 (원본 200x300px) */
function makePages(count: number): PageInfo[] {
  return Array.from({ length: count }, (_, i) => ({
    pageIndex: i,
    width: 200,
    height: 300,
    sectionIndex: 0,
    marginLeft: 10,
    marginRight: 10,
  })) as PageInfo[];
}

// 그리드 배치 기준값 (zoom 0.5, viewport 340):
//   pw=100, ph=150, gap=10 → columns=3, gridWidth=320, marginLeft=10
//   pageLefts: col0=10, col1=120, col2=230 / rowTops: row0=10, row1=170

test('단일 열 모드: getPageAtXY는 getPageAtY와 동일하다 (무회귀)', () => {
  const vs = new VirtualScroll();
  vs.setPageDimensions(makePages(3), 1.0, 340);
  assert.equal(vs.isGridMode(), false);
  for (const y of [0, 15, 320, 640, 2000]) {
    for (const x of [0, 100, 500]) {
      assert.equal(vs.getPageAtXY(x, y), vs.getPageAtY(y));
    }
  }
});

test('줌 0.5 이하 + 페이지 1장: 그리드 미진입', () => {
  const vs = new VirtualScroll();
  vs.setPageDimensions(makePages(1), 0.5, 340);
  assert.equal(vs.isGridMode(), false);
  assert.equal(vs.getPageAtXY(300, 10), 0);
});

test('그리드 모드: 같은 행에서 X로 열을 판정한다', () => {
  const vs = new VirtualScroll();
  vs.setPageDimensions(makePages(6), 0.5, 340);
  assert.equal(vs.isGridMode(), true);
  assert.equal(vs.getColumns(), 3);
  // 1행 (y=20): 각 열 내부 클릭
  assert.equal(vs.getPageAtXY(15, 20), 0);
  assert.equal(vs.getPageAtXY(125, 20), 1);
  assert.equal(vs.getPageAtXY(235, 20), 2);
  // 2행 (y=180)
  assert.equal(vs.getPageAtXY(15, 180), 3);
  assert.equal(vs.getPageAtXY(125, 180), 4);
  assert.equal(vs.getPageAtXY(235, 180), 5);
});

test('그리드 모드: 열 사이 간격/바깥 X는 가장 가까운 페이지로 클램프된다', () => {
  const vs = new VirtualScroll();
  vs.setPageDimensions(makePages(6), 0.5, 340);
  // col0 오른쪽 간격 (x=117): page1 왼쪽(120)까지 3 < page0 오른쪽(110)까지 7
  assert.equal(vs.getPageAtXY(117, 20), 1);
  // 그리드 왼쪽 바깥 (x=2) → 첫 열
  assert.equal(vs.getPageAtXY(2, 20), 0);
  // 그리드 오른쪽 바깥 (x=335) → 마지막 열
  assert.equal(vs.getPageAtXY(335, 180), 5);
});

test('그리드 모드: 행 위 간격의 Y는 기존 getPageAtY 행 판정을 따르되 열은 X로 고른다', () => {
  const vs = new VirtualScroll();
  vs.setPageDimensions(makePages(6), 0.5, 340);
  // 최상단 간격 (y=5) → 1행, x=235 → 3번째 열
  assert.equal(vs.getPageAtXY(235, 5), 2);
  // 행 사이 간격 (y=165, row1 top=170 직전) → 1행 유지
  assert.equal(vs.getPageAtXY(15, 165), 0);
});

test('그리드 모드: 마지막 행이 부분 행이어도 범위를 벗어나지 않는다', () => {
  const vs = new VirtualScroll();
  vs.setPageDimensions(makePages(5), 0.5, 340); // 2행: [0,1,2] + [3,4]
  // 2행 x가 3번째 열 위치여도 마지막 페이지(4)로 클램프
  assert.equal(vs.getPageAtXY(235, 180), 4);
  assert.equal(vs.getPageAtXY(125, 180), 4);
  assert.equal(vs.getPageAtXY(15, 180), 3);
});
