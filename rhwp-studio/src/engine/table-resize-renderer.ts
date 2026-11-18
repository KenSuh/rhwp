import type { VirtualScroll } from '@/view/virtual-scroll';
import type { CellBbox } from '@/core/types';

/** 경계선 종류 */
export type BorderEdgeType = 'row' | 'col';

/** 감지된 경계선 정보 */
export interface BorderEdge {
  type: BorderEdgeType;
  /** 경계선 인덱스 (행: 0=첫 행 상단, 열: 0=첫 열 좌측) */
  index: number;
  pageIndex: number;
}

interface RowLine { y: number; xStart: number; xEnd: number; index: number }
interface ColLine { x: number; yStart: number; yEnd: number; index: number }

function mergeSegments(segments: readonly { readonly start: number; readonly end: number }[]) {
  const merged: { start: number; end: number }[] = [];
  for (const segment of [...segments].sort((a, b) => a.start - b.start)) {
    const previous = merged[merged.length - 1];
    if (previous && segment.start <= previous.end) {
      previous.end = Math.max(previous.end, segment.end);
    } else {
      merged.push({ ...segment });
    }
  }
  return merged;
}

/** 표 셀 경계선 위 hover 시 마커(하이라이트 라인)를 표시한다 */
export class TableResizeRenderer {
  private layer: HTMLDivElement;
  private markers: HTMLDivElement[] = [];
  private static readonly MARKER_COLOR = 'rgba(0, 120, 215, 0.5)';
  private static readonly MARKER_THICKNESS = 3;

  constructor(
    private container: HTMLElement,
    private virtualScroll: VirtualScroll,
  ) {
    this.layer = document.createElement('div');
    this.layer.className = 'table-resize-layer';
    this.layer.style.cssText = 'position:absolute;top:0;left:0;width:100%;height:100%;pointer-events:none;z-index:6;';
    const scrollContent = container.querySelector('#scroll-content');
    if (scrollContent) {
      scrollContent.appendChild(this.layer);
    }
  }

  /** 셀 bbox 배열에서 행/열 경계선 좌표를 계산한다 (페이지 좌표 기준) */
  computeBorderLines(bboxes: CellBbox[]): { rowLines: RowLine[]; colLines: ColLine[] } {
    if (bboxes.length === 0) return { rowLines: [], colLines: [] };

    const ry = (v: number) => Math.round(v * 10) / 10; // 소수점 1자리 반올림

    // 모든 셀의 상/하단, 좌/우측 좌표 수집
    const rowSegments = new Map<number, { start: number; end: number }[]>();
    const colSegments = new Map<number, { start: number; end: number }[]>();
    for (const b of bboxes) {
      for (const y of [ry(b.y), ry(b.y + b.h)]) {
        const segments = rowSegments.get(y) ?? [];
        segments.push({ start: b.x, end: b.x + b.w });
        rowSegments.set(y, segments);
      }
      for (const x of [ry(b.x), ry(b.x + b.w)]) {
        const segments = colSegments.get(x) ?? [];
        segments.push({ start: b.y, end: b.y + b.h });
        colSegments.set(x, segments);
      }
    }

    // 정렬하여 인덱스 부여
    const rowLines: RowLine[] = [...rowSegments].sort(([a], [b]) => a - b).flatMap(([y, segments], index) =>
      mergeSegments(segments)
        .map(segment => ({ y, xStart: segment.start, xEnd: segment.end, index })),
    );

    const colLines: ColLine[] = [...colSegments].sort(([a], [b]) => a - b).flatMap(([x, segments], index) =>
      mergeSegments(segments)
        .map(segment => ({ x, yStart: segment.start, yEnd: segment.end, index })),
    );

    return { rowLines, colLines };
  }

  /** 마우스 좌표가 경계선 위인지 판별한다 (페이지 좌표 기준) */
  hitTestBorder(
    pageX: number, pageY: number,
    bboxes: CellBbox[],
    tolerance = 4,
  ): BorderEdge | null {
    if (bboxes.length === 0) return null;

    const pageIndex = bboxes[0].pageIndex;
    const { rowLines, colLines } = this.computeBorderLines(bboxes.filter(b => b.pageIndex === pageIndex));

    // 행 경계선 검사 (수평선)
    for (const line of rowLines) {
      if (Math.abs(pageY - line.y) <= tolerance &&
          pageX >= line.xStart - tolerance && pageX <= line.xEnd + tolerance) {
        return { type: 'row', index: line.index, pageIndex };
      }
    }

    // 열 경계선 검사 (수직선)
    for (const line of colLines) {
      if (Math.abs(pageX - line.x) <= tolerance &&
          pageY >= line.yStart - tolerance && pageY <= line.yEnd + tolerance) {
        return { type: 'col', index: line.index, pageIndex };
      }
    }

    return null;
  }

  /** 경계선 위에 마커(하이라이트 라인)를 표시한다 */
  showMarker(
    edge: BorderEdge,
    bboxes: CellBbox[],
    zoom: number,
  ): void {
    this.clear();
    this.ensureAttached();
    const pageBboxes = bboxes.filter(b => b.pageIndex === edge.pageIndex);
    if (pageBboxes.length === 0) return;

    const { rowLines, colLines } = this.computeBorderLines(pageBboxes);
    const scrollContent = this.container.querySelector('#scroll-content');
    const contentWidth = scrollContent?.clientWidth ?? 0;
    const pageOffset = this.virtualScroll.getPageOffset(edge.pageIndex);
    const pageDisplayWidth = this.virtualScroll.getPageWidth(edge.pageIndex);
    const pageLeft = (contentWidth - pageDisplayWidth) / 2;

    const t = TableResizeRenderer.MARKER_THICKNESS;
    if (edge.type === 'row') {
      for (const line of rowLines.filter(l => l.index === edge.index)) {
        const left = pageLeft + line.xStart * zoom;
        const top = pageOffset + line.y * zoom - t / 2;
        const width = (line.xEnd - line.xStart) * zoom;
        this.addMarker(
          `position:absolute;` +
          `left:${left}px;top:${top}px;` +
          `width:${width}px;height:${t}px;` +
          `background:${TableResizeRenderer.MARKER_COLOR};pointer-events:none;`,
        );
      }
    } else {
      for (const line of colLines.filter(l => l.index === edge.index)) {
        const left = pageLeft + line.x * zoom - t / 2;
        const top = pageOffset + line.yStart * zoom;
        const height = (line.yEnd - line.yStart) * zoom;
        this.addMarker(
          `position:absolute;` +
          `left:${left}px;top:${top}px;` +
          `width:${t}px;height:${height}px;` +
          `background:${TableResizeRenderer.MARKER_COLOR};pointer-events:none;`,
        );
      }
    }
  }

  /** 드래그 중 마커를 지정된 위치에 표시한다 (원래 경계선이 아닌 마우스 위치) */
  showDragMarker(
    type: BorderEdgeType,
    position: number, // row: pageY, col: pageX
    pageIndex: number,
    bboxes: CellBbox[],
    zoom: number,
  ): void {
    this.clear();
    this.ensureAttached();
    const pageBboxes = bboxes.filter(b => b.pageIndex === pageIndex);
    if (pageBboxes.length === 0) return;

    const scrollContent = this.container.querySelector('#scroll-content');
    const contentWidth = scrollContent?.clientWidth ?? 0;
    const pageOffset = this.virtualScroll.getPageOffset(pageIndex);
    const pageDisplayWidth = this.virtualScroll.getPageWidth(pageIndex);
    const pageLeft = (contentWidth - pageDisplayWidth) / 2;

    const t = TableResizeRenderer.MARKER_THICKNESS;
    if (type === 'row') {
      const minX = Math.min(...pageBboxes.map(b => b.x));
      const maxX = Math.max(...pageBboxes.map(b => b.x + b.w));
      const left = pageLeft + minX * zoom;
      const top = pageOffset + position * zoom - t / 2;
      const width = (maxX - minX) * zoom;
      this.addMarker(
        `position:absolute;left:${left}px;top:${top}px;` +
        `width:${width}px;height:${t}px;` +
        `background:${TableResizeRenderer.MARKER_COLOR};pointer-events:none;`,
      );
    } else {
      const minY = Math.min(...pageBboxes.map(b => b.y));
      const maxY = Math.max(...pageBboxes.map(b => b.y + b.h));
      const left = pageLeft + position * zoom - t / 2;
      const top = pageOffset + minY * zoom;
      const height = (maxY - minY) * zoom;
      this.addMarker(
        `position:absolute;left:${left}px;top:${top}px;` +
        `width:${t}px;height:${height}px;` +
        `background:${TableResizeRenderer.MARKER_COLOR};pointer-events:none;`,
      );
    }
  }

  private addMarker(cssText: string): void {
    const el = document.createElement('div');
    el.style.cssText = cssText;
    this.layer.appendChild(el);
    this.markers.push(el);
  }

  /** 마커를 제거한다 */
  clear(): void {
    for (const marker of this.markers) {
      marker.remove();
    }
    this.markers = [];
  }

  /** 레이어가 DOM에 없으면 재부착한다 */
  private ensureAttached(): void {
    if (this.layer.parentElement) return;
    const scrollContent = this.container.querySelector('#scroll-content');
    if (scrollContent) {
      scrollContent.appendChild(this.layer);
    }
  }

  dispose(): void {
    this.clear();
    this.layer.remove();
  }
}
