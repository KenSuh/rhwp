/** input-handler picture/shape methods — extracted from InputHandler class */
/* eslint-disable @typescript-eslint/no-explicit-any */

import { ObjectGestureRecorder, documentEpochOf } from './command';
import type { EditCommand } from './command';
import type { WasmBridge } from '../core/wasm-bridge';
import { stateLabelOf } from './history';
import { InputHandler } from './input-handler';
import {
  bindPointerHostToWasm,
  ensureGestureMatchesLiveEpoch,
  epochFromLabel,
  hasRawOpenPointerGesture,
  isActiveGestureEpochStale,
} from './input-handler-table';

/** 클릭 좌표에서 그림, 글상자, 수식 개체를 찾는다. */
/** 점과 선분 사이 최소 거리 (px) */
function pointToSegmentDist(px: number, py: number, x1: number, y1: number, x2: number, y2: number): number {
  const dx = x2 - x1, dy = y2 - y1;
  const lenSq = dx * dx + dy * dy;
  if (lenSq === 0) return Math.hypot(px - x1, py - y1);
  let t = ((px - x1) * dx + (py - y1) * dy) / lenSq;
  t = Math.max(0, Math.min(1, t));
  return Math.hypot(px - (x1 + t * dx), py - (y1 + t * dy));
}

function sameCellPath(a?: { controlIndex: number; cellIndex: number; cellParaIndex: number }[], b?: { controlIndex: number; cellIndex: number; cellParaIndex: number }[]): boolean {
  if (!a?.length && !b?.length) return true;
  if (!a?.length || !b?.length || a.length !== b.length) return false;
  return a.every((entry, idx) => {
    const other = b[idx];
    return entry.controlIndex === other.controlIndex &&
      entry.cellIndex === other.cellIndex &&
      entry.cellParaIndex === other.cellParaIndex;
  });
}

function controlMatchesRef(ctrl: any, ref: { sec: number; ppi: number; ci: number; cellPath?: unknown[] }): boolean {
  if (ctrl.secIdx !== ref.sec || ctrl.controlIdx !== ref.ci) return false;
  if (ref.cellPath?.length) {
    return (ctrl.parentParaIdx ?? ctrl.paraIdx) === ref.ppi && sameCellPath(ctrl.cellPath, ref.cellPath as any);
  }
  return ctrl.paraIdx === ref.ppi;
}

export function findPictureAtClick(this: any,
  pageIdx: number, pageX: number, pageY: number,
): { sec: number; ppi: number; ci: number; type: 'image' | 'shape' | 'equation' | 'group' | 'line'; cellIdx?: number; cellParaIdx?: number; cellPath?: any[]; x1?: number; y1?: number; x2?: number; y2?: number } | null {
  bindPointerHostToWasm(this);
  if (this?.wasm && !hasRawOpenPointerGesture(this)) {
    this._gestureOwnerEpoch = documentEpochOf(this.wasm);
  }
  try {
    const layout = this.wasm.getPageControlLayout(pageIdx);
    for (const ctrl of layout.controls) {
      if (ctrl.type !== 'image' && ctrl.type !== 'shape' && ctrl.type !== 'equation' && ctrl.type !== 'group' && ctrl.type !== 'line') continue;
      if (ctrl.secIdx === undefined || ctrl.paraIdx === undefined || ctrl.controlIdx === undefined) continue;

      if (ctrl.type === 'line') {
        // 직선: 점-선분 거리, 연결선: 곡선 경로 샘플링으로 히트 판정
        const threshold = 6;
        const dist1 = pointToSegmentDist(pageX, pageY, ctrl.x1, ctrl.y1, ctrl.x2, ctrl.y2);
        let hit = dist1 <= threshold;
        if (!hit && ctrl.w > 2 && ctrl.h > 2) {
          const sx = ctrl.x1, sy = ctrl.y1, ex = ctrl.x2, ey = ctrl.y2;
          const mx = ctrl.x + ctrl.w / 2, my = ctrl.y + ctrl.h / 2;
          // 꺽인 연결선: 가능한 모든 직각 경로 검사
          const segs: [number,number,number,number][] = [
            // 수평→수직→수평 (S자 꺽임)
            [sx,sy, mx,sy], [mx,sy, mx,ey], [mx,ey, ex,ey],
            // 수직→수평→수직 (S자 꺽임)
            [sx,sy, sx,my], [sx,my, ex,my], [ex,my, ex,ey],
            // L자 꺽임
            [sx,sy, ex,sy], [ex,sy, ex,ey],
            [sx,sy, sx,ey], [sx,ey, ex,ey],
          ];
          for (const [ax,ay,bx,by] of segs) {
            if (pointToSegmentDist(pageX, pageY, ax, ay, bx, by) <= threshold) {
              hit = true; break;
            }
          }
          // 곡선 연결선: 베지어 곡선 — 8세그먼트 샘플링
          if (!hit) {
            const c1x = mx, c1y = sy, c2x = mx, c2y = ey;
            const N = 8;
            let prevX = sx, prevY = sy;
            for (let k = 1; k <= N; k++) {
              const t = k / N;
              const u = 1 - t;
              const bx = u*u*u*sx + 3*u*u*t*c1x + 3*u*t*t*c2x + t*t*t*ex;
              const by = u*u*u*sy + 3*u*u*t*c1y + 3*u*t*t*c2y + t*t*t*ey;
              if (pointToSegmentDist(pageX, pageY, prevX, prevY, bx, by) <= threshold) {
                hit = true; break;
              }
              prevX = bx; prevY = by;
            }
          }
        }
        if (hit) {
          return { sec: ctrl.secIdx, ppi: ctrl.paraIdx, ci: ctrl.controlIdx, type: 'line',
            x1: ctrl.x1, y1: ctrl.y1, x2: ctrl.x2, y2: ctrl.y2 };
        }
      } else {
        // bbox 히트 판정
	        if (pageX >= ctrl.x && pageX <= ctrl.x + ctrl.w &&
	            pageY >= ctrl.y && pageY <= ctrl.y + ctrl.h) {
	          return { sec: ctrl.secIdx, ppi: ctrl.parentParaIdx ?? ctrl.paraIdx, ci: ctrl.controlIdx, type: ctrl.type, cellIdx: ctrl.cellIdx, cellParaIdx: ctrl.cellParaIdx, cellPath: ctrl.cellPath };
	        }
      }
    }
  } catch { /* ignore */ }
  return null;
}

/** 선택된 개체의 bbox를 페이지 레이아웃에서 찾는다. */
export function findPictureBbox(this: any,
	  ref: { sec: number; ppi: number; ci: number; type?: 'image' | 'shape' | 'equation' | 'group' | 'line'; cellIdx?: number; cellParaIdx?: number; cellPath?: any[] },
): { pageIndex: number; x: number; y: number; w: number; h: number; x1?: number; y1?: number; x2?: number; y2?: number } | null {
  bindPointerHostToWasm(this);
  if (this?.wasm && !hasRawOpenPointerGesture(this)) {
    this._gestureOwnerEpoch = documentEpochOf(this.wasm);
  }
  const matchType = ref.type ?? 'image';
  // line은 shape의 하위 타입 → layout에서 'line'으로 반환됨
  const layoutType = matchType === 'line' ? 'line' : matchType;
  try {
    const pageCount = this.wasm.pageCount;
    for (let p = 0; p < pageCount; p++) {
      const layout = this.wasm.getPageControlLayout(p);
      for (const ctrl of layout.controls) {
	        if (ctrl.type === layoutType && controlMatchesRef(ctrl, ref)) {
          // 표 셀 내 수식: cellIdx/cellParaIdx도 매칭
          if (matchType === 'equation' && ref.cellIdx !== undefined) {
            if (ctrl.cellIdx !== ref.cellIdx || ctrl.cellParaIdx !== ref.cellParaIdx) continue;
          }
          return { pageIndex: p, x: ctrl.x, y: ctrl.y, w: ctrl.w, h: ctrl.h,
            x1: ctrl.x1, y1: ctrl.y1, x2: ctrl.x2, y2: ctrl.y2 };
        }
      }
    }
  } catch { /* ignore */ }
  return null;
}

/** 개체 선택 시 외곽선 + 핸들을 렌더링한다. */
export function renderPictureObjectSelection(this: any): void {
  if (!this.pictureObjectRenderer) return;

  // 다중 선택: 합산 bbox로 핸들 표시
  if (this.cursor.isMultiPictureSelection()) {
    const refs = this.cursor.getSelectedPictureRefs();
    try {
      const zoom = this.viewportManager.getZoom();
      let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
      let pageIndex = 0;
      for (const r of refs) {
        const bbox = this.findPictureBbox(r);
        if (bbox) {
          pageIndex = bbox.pageIndex;
          minX = Math.min(minX, bbox.x);
          minY = Math.min(minY, bbox.y);
          maxX = Math.max(maxX, bbox.x + bbox.w);
          maxY = Math.max(maxY, bbox.y + bbox.h);
        }
      }
      if (minX < Infinity) {
        this.pictureObjectRenderer.render(
          { pageIndex, x: minX, y: minY, width: maxX - minX, height: maxY - minY },
          zoom,
        );
      } else {
        this.pictureObjectRenderer.clear();
      }
    } catch {
      this.pictureObjectRenderer.clear();
    }
    return;
  }

  const ref = this.cursor.getSelectedPictureRef();
  if (!ref) {
    this.pictureObjectRenderer.clear();
    return;
  }
  const matchType = ref.type ?? 'image';
  const layoutType = matchType === 'line' ? 'line' : matchType;
  try {
    const zoom = this.viewportManager.getZoom();
    const pageCount = this.wasm.pageCount;
    for (let p = 0; p < pageCount; p++) {
      const layout = this.wasm.getPageControlLayout(p);
      for (const ctrl of layout.controls) {
	        if (ctrl.type === layoutType && controlMatchesRef(ctrl, ref)) {
          // 표 셀 내 수식: cellIdx/cellParaIdx도 매칭
          if (matchType === 'equation' && ref.cellIdx !== undefined) {
            if (ctrl.cellIdx !== ref.cellIdx || ctrl.cellParaIdx !== ref.cellParaIdx) continue;
          }

          if (matchType === 'line') {
            // 직선/연결선: 시작점/끝점 핸들 (꺽인/곡선 연결선은 중간점 추가)
            let midPoint: { x: number; y: number } | undefined;
            try {
              const props = this.wasm.getShapeProperties(ref.sec, ref.ppi, ref.ci);
              // connectorType >= 3: 꺽인(3~5) 또는 곡선(6~8)
              if (props.connectorType !== undefined && props.connectorType >= 3) {
                if (props.connectorMidX !== undefined && props.connectorMidY !== undefined) {
                  // 실제 꺽임/곡선 제어점 좌표 (HWPUNIT → page px)
                  const PX = 96 / 7200;
                  midPoint = {
                    x: ctrl.x + props.connectorMidX * PX,
                    y: ctrl.y + props.connectorMidY * PX,
                  };
                } else {
                  midPoint = { x: (ctrl.x1 + ctrl.x2) / 2, y: (ctrl.y1 + ctrl.y2) / 2 };
                }
              }
            } catch { /* 일반 선 */ }
            this.pictureObjectRenderer.renderLine(
              { pageIndex: p, x1: ctrl.x1, y1: ctrl.y1, x2: ctrl.x2, y2: ctrl.y2,
                x: ctrl.x, y: ctrl.y, width: ctrl.w, height: ctrl.h },
              zoom,
              midPoint,
            );
            return;
          }

          const bx = ctrl.x, by = ctrl.y, bw = ctrl.w, bh = ctrl.h;

          // 회전각 조회 (shape만)
          let rotAngle = 0;
          if (ref.type === 'shape') {
            try {
              const props = this.wasm.getShapeProperties(ref.sec, ref.ppi, ref.ci);
              rotAngle = (props.rotationAngle as number) ?? 0;
            } catch { /* ignore */ }
          }

          this.pictureObjectRenderer.render(
            { pageIndex: p, x: bx, y: by, width: bw, height: bh },
            zoom,
            rotAngle,
          );
          return;
        }
      }
    }
    this.pictureObjectRenderer.clear();
  } catch (e) {
    console.warn('[InputHandler] renderPictureObjectSelection 실패:', e);
    this.pictureObjectRenderer.clear();
  }
}

export function exitPictureObjectSelectionIfNeeded(this: any): void {
  if (this.cursor.isInPictureObjectSelection()) {
    this.cursor.exitPictureObjectSelection();
    this.pictureObjectRenderer?.clear();
    this.eventBus.emit('picture-object-selection-changed', false);
  }
}

/** 클릭 좌표가 글상자의 경계선 위인지 판정한다. */
export function isShapeBorderClick(this: any,
  pageX: number, pageY: number,
  shape: { sec: number; ppi: number; ci: number },
): boolean {
  const THRESHOLD = 3; // px
  const bbox = findPictureBbox.call(this, { ...shape, type: 'shape' as const });
  if (!bbox) return false;
  const dx = Math.min(pageX - bbox.x, bbox.x + bbox.w - pageX);
  const dy = Math.min(pageY - bbox.y, bbox.y + bbox.h - pageY);
  return dx <= THRESHOLD || dy <= THRESHOLD;
}

// ─── 개체 속성 조회 헬퍼 (그림/글상자 분기) ──────────────

/** 개체 속성을 타입에 따라 조회한다. */
export function getObjectProperties(this: any, ref: { sec: number; ppi: number; ci: number; type: string; cellPath?: any[] }): any {
  bindPointerHostToWasm(this);
  if (this && !hasRawOpenPointerGesture(this) && this.wasm) {
    this._gestureOwnerEpoch = documentEpochOf(this.wasm);
  }
  if (ref.type === 'shape' || ref.type === 'line' || ref.type === 'group') {
    return this.wasm.getShapeProperties(ref.sec, ref.ppi, ref.ci);
  }
  if (ref.type === 'image' && ref.cellPath?.length) {
    return this.wasm.getPicturePropertiesByPath(ref.sec, ref.ppi, JSON.stringify(ref.cellPath), ref.ci);
  }
  return this.wasm.getPictureProperties(ref.sec, ref.ppi, ref.ci);
}

/** 개체 속성을 타입에 따라 변경한다. */
export function setObjectProperties(this: any, ref: { sec: number; ppi: number; ci: number; type: string; cellPath?: any[] }, props: Record<string, unknown>): void {
  if (ref.type === 'shape' || ref.type === 'line' || ref.type === 'group') {
    this.wasm.setShapeProperties(ref.sec, ref.ppi, ref.ci, props);
  } else if (ref.type === 'image' && ref.cellPath?.length) {
    this.wasm.setPicturePropertiesByPath(ref.sec, ref.ppi, JSON.stringify(ref.cellPath), ref.ci, props);
  } else {
    this.wasm.setPictureProperties(ref.sec, ref.ppi, ref.ci, props);
  }
}

/** 개체를 타입에 따라 삭제한다. */
export function deleteObjectControl(this: any, ref: { sec: number; ppi: number; ci: number; type: 'image' | 'shape' | 'equation' | 'group' | 'line'; cellPath?: any[] }): void {
  if (ref.type === 'shape' || ref.type === 'group' || ref.type === 'line') {
    this.wasm.deleteShapeControl(ref.sec, ref.ppi, ref.ci);
  } else if (ref.cellPath?.length) {
    this.wasm.deletePictureControlByPath(ref.sec, ref.ppi, JSON.stringify(ref.cellPath), ref.ci);
  } else {
    this.wasm.deletePictureControl(ref.sec, ref.ppi, ref.ci);
  }
}

// ─── 핸들 드래그 리사이즈 ─────────────────────────

/** 1 page px = 7200/96 = 75 HWPUNIT */
const PX_TO_HWP = 7200 / 96;
const MIN_SIZE_HWP = 283; // ≈1mm
const MIN_NATIVE_SIZE_HWP = 200;

function resolveNativeDimension(orig: number, clamped: number): number {
  return clamped === orig ? orig : Math.max(clamped, MIN_NATIVE_SIZE_HWP);
}

type SingleResizeState = {
  readonly origWidth: number; readonly origHeight: number;
  origHorzOffset?: number; origVertOffset?: number;
  readonly dir: string; readonly rotationAngle?: number;
  readonly startClientX: number; readonly startClientY: number;
  readonly ref: { readonly type: string };
  readonly bbox: { readonly x: number; readonly y: number; readonly w: number; readonly h: number };
};

type MultiResizeRef = Parameters<typeof setObjectProperties>[0] & {
  readonly origWidth: number;
  readonly origHeight: number;
  readonly origHorzOffset: number;
  readonly origVertOffset: number;
  readonly bboxX: number;
  readonly bboxY: number;
};

function singleResizeTarget(
  state: SingleResizeState,
  resized: ReturnType<typeof calcResizedBboxRotated>,
  getOriginal: () => { readonly horzOffset: number; readonly vertOffset: number },
) {
  if (state.origHorzOffset === undefined || state.origVertOffset === undefined) {
    const original = getOriginal();
    state.origHorzOffset ??= original.horzOffset;
    state.origVertOffset ??= original.vertOffset;
  }
  const { nativeWidth: width, nativeHeight: height } = resized;
  const horzOffset = ((state.origHorzOffset | 0) + resized.deltaH) >>> 0;
  const vertOffset = ((state.origVertOffset | 0) + resized.deltaV) >>> 0;
  return { props: { width, height, horzOffset, vertOffset },
    changed: width !== state.origWidth || height !== state.origHeight
      || horzOffset !== state.origHorzOffset || vertOffset !== state.origVertOffset };
}

function multiResizeTargets(
  refs: readonly MultiResizeRef[],
  dir: string,
  bbox: { readonly x: number; readonly y: number; readonly w: number; readonly h: number },
  resized: { readonly x: number; readonly y: number; readonly width: number; readonly height: number },
) {
  const baseW = bbox.w || Math.max(1, ...refs.map(ref => ref.origWidth)) / PX_TO_HWP;
  const baseH = bbox.h || Math.max(1, ...refs.map(ref => ref.origHeight)) / PX_TO_HWP;
  const sx = dir.includes('e') || dir.includes('w') ? 1 + (resized.width - bbox.w) / baseW : 1;
  const sy = dir.includes('n') || dir.includes('s') ? 1 + (resized.height - bbox.h) / baseH : 1;
  return refs.map(ref => {
    const clampedW = Math.max(ref.origWidth + Math.round(ref.origWidth * (sx - 1)), Math.min(MIN_SIZE_HWP, ref.origWidth));
    const clampedH = Math.max(ref.origHeight + Math.round(ref.origHeight * (sy - 1)), Math.min(MIN_SIZE_HWP, ref.origHeight));
    const width = ref.type === 'image' ? clampedW : resolveNativeDimension(ref.origWidth, clampedW);
    const height = ref.type === 'image' ? clampedH : resolveNativeDimension(ref.origHeight, clampedH);
    const deltaH = Math.round((resized.x + (ref.bboxX - bbox.x) * sx - ref.bboxX) * PX_TO_HWP)
      - (dir.includes('w') ? width - clampedW : 0);
    const deltaV = Math.round((resized.y + (ref.bboxY - bbox.y) * sy - ref.bboxY) * PX_TO_HWP)
      - (dir.includes('n') ? height - clampedH : 0);
    return {
      ref,
      changed: width !== ref.origWidth || height !== ref.origHeight || deltaH !== 0 || deltaV !== 0,
      props: { width, height, horzOffset: ((ref.origHorzOffset | 0) + deltaH) >>> 0, vertOffset: ((ref.origVertOffset | 0) + deltaV) >>> 0 },
    };
  });
}

function restoreRecorderBefore(wasm: WasmBridge, recorder: ObjectGestureRecorder): boolean {
  if (!recorder.started || typeof wasm.restoreSnapshot !== 'function') return false;
  // The recorder owns the snapshot; restoring it avoids native dimension normalization.
  const before: unknown = Reflect.get(recorder, 'before');
  if (!before || typeof before !== 'object' || !('id' in before) || typeof before.id !== 'number'
    || !('label' in before) || !('epoch' in before) || before.epoch !== documentEpochOf(wasm)
    || stateLabelOf(wasm) === before.label) return false;
  const tempId = typeof wasm.saveSnapshot === 'function' ? wasm.saveSnapshot() : null;
  try {
    wasm.restoreSnapshot(before.id);
    return true;
  } finally {
    if (tempId !== null) wasm.discardSnapshot(tempId);
  }
}

function resizeProperties(
  original: { readonly origWidth: number; readonly origHeight: number },
  target: ReturnType<typeof singleResizeTarget>['props'],
  restored: boolean,
): Record<string, number> {
  // Each caller has begun recording; offset writes connect a return to the original label.
  const props: Record<string, number> = { horzOffset: target.horzOffset, vertOffset: target.vertOffset };
  if (!restored || target.width !== original.origWidth) props['width'] = target.width;
  if (!restored || target.height !== original.origHeight) props['height'] = target.height;
  return props;
}

/**
 * 회전각을 반영하여 리사이즈 후 새 bbox(비회전 기준)를 계산한다.
 * - 마우스 delta를 도형 로컬 좌표계로 역변환한다.
 * - 반대편 꼭짓점(pivot)이 page 좌표에서 고정되도록 중심을 재계산한다.
 */
function calcResizedBboxRotated(state: SingleResizeState, e: MouseEvent, zoom: number) {
  const angleDeg = state.rotationAngle ?? 0;
  const rad = angleDeg * Math.PI / 180;
  const cosA = Math.cos(rad);
  const sinA = Math.sin(rad);

  const dx = (e.clientX - state.startClientX) / zoom;
  const dy = (e.clientY - state.startClientY) / zoom;

  // 화면 좌표 delta → 도형 로컬 좌표계로 역변환
  const localDx = dx * cosA + dy * sinA;
  const localDy = -dx * sinA + dy * cosA;

  const w0 = state.bbox.w;
  const h0 = state.bbox.h;
  const dir: string = state.dir;

  // 크기 제한 없이 마우스 이동 반영 (반대편으로 넘어가면 음수 발생 가능)
  let valW = w0;
  let valH = h0;
  if (dir.includes('e')) valW = w0 + localDx;
  if (dir.includes('w')) valW = w0 - localDx;
  if (dir.includes('s')) valH = h0 + localDy;
  if (dir.includes('n')) valH = h0 - localDy;

  // Resolve setter normalization before deriving the opposite native pivot.
  const clampedW = Math.max(state.origWidth + Math.round((Math.abs(valW) - w0) * PX_TO_HWP), Math.min(MIN_SIZE_HWP, state.origWidth));
  const clampedH = Math.max(state.origHeight + Math.round((Math.abs(valH) - h0) * PX_TO_HWP), Math.min(MIN_SIZE_HWP, state.origHeight));
  // Picture setters store dimensions directly; only shape/group setters impose 200 HWP.
  const nativeWidth = state.ref.type === 'image' ? clampedW : resolveNativeDimension(state.origWidth, clampedW);
  const nativeHeight = state.ref.type === 'image' ? clampedH : resolveNativeDimension(state.origHeight, clampedH);
  const flipX = valW < 0 && Math.round(localDx * PX_TO_HWP) !== 0;
  const flipY = valH < 0 && Math.round(localDy * PX_TO_HWP) !== 0;
  const x0 = dir.includes('e') ? -state.origWidth / 2 : (dir.includes('w') ? state.origWidth / 2 : 0);
  const y0 = dir.includes('s') ? -state.origHeight / 2 : (dir.includes('n') ? state.origHeight / 2 : 0);
  const x1 = (dir.includes('e') ? -nativeWidth / 2 : (dir.includes('w') ? nativeWidth / 2 : 0)) * (flipX ? -1 : 1);
  const y1 = (dir.includes('s') ? -nativeHeight / 2 : (dir.includes('n') ? nativeHeight / 2 : 0)) * (flipY ? -1 : 1);
  const deltaH = Math.round((state.origWidth - nativeWidth) / 2 + (x0 - x1) * cosA - (y0 - y1) * sinA);
  const deltaV = Math.round((state.origHeight - nativeHeight) / 2 + (x0 - x1) * sinA + (y0 - y1) * cosA);
  return { x: state.bbox.x + deltaH / PX_TO_HWP, y: state.bbox.y + deltaV / PX_TO_HWP,
    width: nativeWidth / PX_TO_HWP, height: nativeHeight / PX_TO_HWP, nativeWidth, nativeHeight, deltaH, deltaV };
}

export function updatePictureResizeDrag(this: any, e: MouseEvent): void {
  if (!ensureGestureMatchesLiveEpoch(this) || !this.pictureResizeState) return;
  const zoom = this.viewportManager.getZoom();
  const state = this.pictureResizeState;
  state.epoch ??= this.wasm ? documentEpochOf(this.wasm) : null;
  state.lastClientX = e.clientX;
  state.lastClientY = e.clientY;
  const expectedEpoch = state.epoch ?? epochFromLabel(state.beforeLabel) ?? this._gestureOwnerEpoch ?? null;

  // 핸들은 고정, 예비 테두리만 갱신
  const rotAngle = (state.rotationAngle ?? 0) as number;
  const newBbox = state.multiRefs
    ? (this.calcResizedBbox ?? calcResizedBbox).call(this, e, zoom)
    : calcResizedBboxRotated(state, e, zoom);
  const targets = state.multiRefs?.length ? multiResizeTargets(state.multiRefs, state.dir, state.bbox, newBbox) : null;
  let { x, y, width, height } = newBbox;
  if (targets) {
    const boxes = targets.map(({ ref, props }) => ({
      x: ref.bboxX + ((props.horzOffset | 0) - (ref.origHorzOffset | 0)) / PX_TO_HWP,
      y: ref.bboxY + ((props.vertOffset | 0) - (ref.origVertOffset | 0)) / PX_TO_HWP,
      w: props.width / PX_TO_HWP, h: props.height / PX_TO_HWP,
    }));
    x = Math.min(...boxes.map(box => box.x));
    y = Math.min(...boxes.map(box => box.y));
    width = Math.max(...boxes.map(box => box.x + box.w)) - x;
    height = Math.max(...boxes.map(box => box.y + box.h)) - y;
  }

  // 모든 경우에 점선 프리뷰만 갱신하여 앵커는 제자리에 머물게 함
  this.pictureObjectRenderer?.renderDragPreview(
    { pageIndex: state.pageIndex, x, y, width, height },
    zoom,
    rotAngle,
  );

  // 다중 선택: 드래그 중 실시간으로 개체 크기/위치 반영
  if (targets) {
    const anyChanged = Boolean(state.recorder?.started) || targets.some(target => target.changed);
    if (!anyChanged) return;
    try {
      const restored = !state.recorder?.started || restoreRecorderBefore(this.wasm, state.recorder);
      state.recorder ??= new ObjectGestureRecorder(
        'resizeObjects',
        { sectionIndex: state.ref.sec, paragraphIndex: state.ref.ppi, charOffset: 0 },
        expectedEpoch,
      );
      if (!state.recorder.begin(this.wasm)) {
        InputHandler.prototype.settleOpenPointerGesture.call(this, 'cancel');
        return;
      }
      for (const { ref, props } of targets) setObjectProperties.call(this, ref, resizeProperties(ref, props, restored));
      try { this.wasm.updateConnectorsInSection(state.ref.sec); } catch { /* ignore */ }
      this.eventBus?.emit('document-changed');
    } catch { /* ignore */ }
  }

  // 그룹 단일 선택: 드래그 중 실시간 크기 반영
  if (!state.multiRefs && state.ref.type === 'group') {
    const target = singleResizeTarget(state, newBbox, () => (this.getObjectProperties ?? getObjectProperties).call(this, state.ref));
    const groupChanged = Boolean(state.recorder?.started) || target.changed;
    if (!groupChanged) return;
    try {
      const restored = !state.recorder?.started || restoreRecorderBefore(this.wasm, state.recorder);
      state.recorder ??= new ObjectGestureRecorder(
        'resizeObjects',
        { sectionIndex: state.ref.sec, paragraphIndex: state.ref.ppi, charOffset: 0 },
        expectedEpoch,
      );
      if (!state.recorder.begin(this.wasm)) {
        InputHandler.prototype.settleOpenPointerGesture.call(this, 'cancel');
        return;
      }
      setObjectProperties.call(this, state.ref, resizeProperties(state, target.props, restored));
      try { this.wasm.updateConnectorsInSection(state.ref.sec); } catch { /* ignore */ }
      this.eventBus?.emit('document-changed');
    } catch { /* ignore */ }
  }
}

export function finishPictureResizeDrag(this: any, e?: MouseEvent): void {
  if (this.pictureResizeState && isActiveGestureEpochStale(this)) {
    InputHandler.prototype.settleOpenPointerGesture.call(this, 'cancel');
    return;
  }
  const clean = () => (this.cleanupPictureResizeDrag ?? cleanupPictureResizeDrag).call(this);
  const state = this.pictureResizeState;
  if (!state || !state.ref || !state.bbox || !this.viewportManager) {
    try {
      state?.recorder?.cancel(this.wasm);
    } finally {
      clean();
    }
    return;
  }

  let command: EditCommand | null = null;
  try {
    state.epoch ??= this.wasm ? documentEpochOf(this.wasm) : null;
    const expectedEpoch = state.epoch ?? epochFromLabel(state.beforeLabel) ?? this._gestureOwnerEpoch ?? null;
    const effectiveEvent = e ?? ({
      clientX: state.lastClientX ?? state.startClientX,
      clientY: state.lastClientY ?? state.startClientY,
    } as MouseEvent);
    const zoom = this.viewportManager.getZoom();

    const pointerMoved = Boolean(
      state.recorder?.started
      || effectiveEvent.clientX !== state.startClientX
      || effectiveEvent.clientY !== state.startClientY,
    );

    // 다중 선택 리사이즈: 드래그 중 실시간 반영 완료 → 최종 확정 + 히스토리 항목 하나로 봉인
    if (pointerMoved && state.multiRefs && state.multiRefs.length > 0) {
      const newBbox = (this.calcResizedBbox ?? calcResizedBbox).call(this, effectiveEvent, zoom);
      const targets = multiResizeTargets(state.multiRefs, state.dir, state.bbox, newBbox);
      const anyChanged = Boolean(state.recorder?.started) || targets.some(target => target.changed);

      if (anyChanged) {
        try {
          const restored = !state.recorder?.started || restoreRecorderBefore(this.wasm, state.recorder);
          state.recorder ??= new ObjectGestureRecorder(
            'resizeObjects',
            { sectionIndex: state.ref.sec, paragraphIndex: state.ref.ppi, charOffset: 0 },
            expectedEpoch,
          );
          if (!state.recorder.begin(this.wasm)) {
            InputHandler.prototype.settleOpenPointerGesture.call(this, 'cancel');
            return;
          }
          for (const { ref, props } of targets) setObjectProperties.call(this, ref, resizeProperties(ref, props, restored));
          try { this.wasm.updateConnectorsInSection(state.ref.sec); } catch { /* ignore */ }
          this.eventBus?.emit('document-changed');
        } catch (err) {
          console.warn('[InputHandler] 다중 개체 리사이즈 실패:', err);
        }
      }
    } else if (pointerMoved && (!state.multiRefs || state.multiRefs.length === 0)) {
      // 단일 선택 리사이즈 (회전 반영: pivot 고정, 위치도 갱신)
      const newBbox = calcResizedBboxRotated(state, effectiveEvent, zoom);
      const { props, changed } = singleResizeTarget(state, newBbox, () => (this.getObjectProperties ?? getObjectProperties).call(this, state.ref));

      try {
        if (state.recorder?.started || changed) {
          const restored = !state.recorder?.started || restoreRecorderBefore(this.wasm, state.recorder);
          state.recorder ??= new ObjectGestureRecorder(
            'resizeObjects',
            { sectionIndex: state.ref.sec, paragraphIndex: state.ref.ppi, charOffset: 0 },
            expectedEpoch,
          );
          if (!state.recorder.begin(this.wasm)) {
            InputHandler.prototype.settleOpenPointerGesture.call(this, 'cancel');
            return;
          }
          setObjectProperties.call(this, state.ref, resizeProperties(state, props, restored));
          try { this.wasm.updateConnectorsInSection(state.ref.sec); } catch { /* ignore */ }
          this.eventBus?.emit('document-changed');
        }
      } catch (err) {
        console.warn('[InputHandler] 개체 리사이즈 실패:', err);
      }
    }

    command = state.recorder?.finish(this.wasm) ?? null;
    if (command) this.history?.recordWithoutExecute(command, this.wasm);
    command = null;
  } finally {
    try {
      command?.discard?.(this.wasm);
      state.recorder?.cancel(this.wasm);
    } finally {
      clean();
    }
  }
  this.renderPictureObjectSelection?.();
}

export function calcResizedBbox(this: any, e: MouseEvent, zoom: number): { x: number; y: number; width: number; height: number } {
  const s = this.pictureResizeState!;
  if (!s.multiRefs) return calcResizedBboxRotated(s, e, zoom);
  const dx = (e.clientX - s.startClientX) / zoom; // page px
  const dy = (e.clientY - s.startClientY) / zoom;
  const minW = Math.min(1, s.bbox.w);
  const minH = Math.min(1, s.bbox.h);

  let { x, y, w, h } = s.bbox;
  const dir = s.dir;

  // 가로 크기 및 위치 계산 (Flip 허용)
  if (dir.includes('e')) {
    const valW = s.bbox.w + dx;
    w = Math.max(Math.abs(valW), minW);
    if (valW < 0) x = s.bbox.x - w; // 반대편으로 넘어가면 시작점 이동
  } else if (dir.includes('w')) {
    const valW = s.bbox.w - dx;
    w = Math.max(Math.abs(valW), minW);
    if (valW >= 0) x = s.bbox.x + (s.bbox.w - w);
    else x = s.bbox.x + s.bbox.w; // 반대편으로 넘어가면 오른쪽 끝이 시작점
  }

  // 세로 크기 및 위치 계산 (Flip 허용)
  if (dir.includes('s')) {
    const valH = s.bbox.h + dy;
    h = Math.max(Math.abs(valH), minH);
    if (valH < 0) y = s.bbox.y - h;
  } else if (dir.includes('n')) {
    const valH = s.bbox.h - dy;
    h = Math.max(Math.abs(valH), minH);
    if (valH >= 0) y = s.bbox.y + (s.bbox.h - h);
    else y = s.bbox.y + s.bbox.h;
  }

  return { x, y, width: w, height: h };
}

export function cleanupPictureResizeDrag(this: any): void {
  this._gestureOwnerEpoch = null;
  this.isPictureResizeDragging = false;
  this.pictureResizeState = null;
  if (this.container?.style) this.container.style.cursor = '';
  if (this.dragRafId) {
    cancelAnimationFrame(this.dragRafId);
    this.dragRafId = 0;
  }
  this.pictureObjectRenderer?.clearDragPreview();
}

export function updatePictureMoveDrag(this: any, e: MouseEvent): void {
  if (!ensureGestureMatchesLiveEpoch(this) || !this.pictureMoveState) return;
  this.pictureMoveState.epoch ??= this.wasm ? documentEpochOf(this.wasm) : null;
  const zoom = this.viewportManager.getZoom();
  const sc = this.container.querySelector('#scroll-content');
  if (!sc) return;
  const cr = sc.getBoundingClientRect();
  const cx = e.clientX - cr.left;
  const cy = e.clientY - cr.top;
  // 이동 delta는 시작 페이지 좌표계에 고정 — grid(다중 열)에서 열을 재판정하면
  // 열 경계 통과 시 원점이 바뀌어 delta가 튄다 (히트테스트와 달리 드래그는 고정이 맞다)
  const pi = this.pictureMoveState.pageIndex;
  const po = this.virtualScroll.getPageOffset(pi);
  const pw = this.virtualScroll.getPageWidth(pi);
  const pl = this.virtualScroll.getPageLeft(pi) >= 0
    ? this.virtualScroll.getPageLeft(pi)
    : (sc.clientWidth - pw) / 2;
  const px = (cx - pl) / zoom;
  const py = (cy - po) / zoom;

  const deltaXpx = px - this.pictureMoveState.lastPageX;
  const deltaYpx = py - this.pictureMoveState.lastPageY;
  const deltaH = Math.round(deltaXpx * 75); // 1 page px = 75 HWPUNIT
  const deltaV = Math.round(deltaYpx * 75);

  if (deltaH === 0 && deltaV === 0) return;

  try {
    // 다중 선택: 모든 개체를 동일 delta로 이동
    const targets = this.pictureMoveState.multiRefs || [this.pictureMoveState.ref];
    const expectedEpoch = this.pictureMoveState.epoch
      ?? epochFromLabel(this.pictureMoveState.beforeLabel)
      ?? this._gestureOwnerEpoch
      ?? null;
    // 제스처 이전 문서·라벨은 첫 실제 변이 직전에 한 번 봉인한다 —
    // 선택된 모든 개체와 연결선 갱신이 하나의 히스토리 항목에 담긴다
    this.pictureMoveState.recorder ??= new ObjectGestureRecorder('moveObjects', {
      sectionIndex: targets[0].sec, paragraphIndex: targets[0].ppi, charOffset: 0,
    }, expectedEpoch);
    if (!this.pictureMoveState.recorder.begin(this.wasm)) {
      InputHandler.prototype.settleOpenPointerGesture.call(this, 'cancel');
      return;
    }
    for (const ref of targets) {
      const props = getObjectProperties.call(this, ref);
      setObjectProperties.call(this, ref, {
        horzOffset: ((props.horzOffset + deltaH) >>> 0),
        vertOffset: ((props.vertOffset + deltaV) >>> 0),
      });
    }
    this.pictureMoveState.lastPageX = px;
    this.pictureMoveState.lastPageY = py;
    this.pictureMoveState.totalDeltaH += deltaH;
    this.pictureMoveState.totalDeltaV += deltaV;
    // 연결선 자동 추적
    try { this.wasm.updateConnectorsInSection(targets[0].sec); } catch { /* ignore */ }
    this.eventBus?.emit('document-changed');
    this.renderPictureObjectSelection?.();
  } catch (err) {
    console.warn('[InputHandler] 개체 이동 드래그 실패:', err);
  }
}

export function finishPictureMoveDrag(this: any): void {
  if (this.pictureMoveState && isActiveGestureEpochStale(this)) {
    InputHandler.prototype.settleOpenPointerGesture.call(this, 'cancel');
    return;
  }
  let command: EditCommand | null = null;
  try {
    if (this.pictureMoveState) {
      // 드래그 전체(모든 대상 + 연결선)를 실제 이전/이후 라벨 쌍을 가진 항목 하나로 기록
      command = this.pictureMoveState.recorder?.finish(this.wasm) ?? null;
      if (command) this.history?.recordWithoutExecute(command, this.wasm);
      command = null;
    }
  } finally {
    try {
      command?.discard?.(this.wasm);
      this.pictureMoveState?.recorder?.cancel(this.wasm);
    } finally {
      this._gestureOwnerEpoch = null;
      this.isPictureMoveDragging = false;
      this.pictureMoveState = null;
      if (this.container?.style) this.container.style.cursor = '';
      if (this.dragRafId) {
        cancelAnimationFrame(this.dragRafId);
        this.dragRafId = 0;
      }
    }
  }
}

// ─── 회전 드래그 ─────────────────────────────────

/** 회전 드래그 중: 마우스 각도에 따라 실시간 회전 적용 */
export function updatePictureRotateDrag(this: any, e: MouseEvent): void {
  if (!ensureGestureMatchesLiveEpoch(this) || !this.pictureRotateState) return;
  const sc = this.container.querySelector('#scroll-content');
  if (!sc) return;
  const cr = sc.getBoundingClientRect();
  const mx = e.clientX - cr.left;
  const my = e.clientY - cr.top;

  const s = this.pictureRotateState;
  s.epoch ??= this.wasm ? documentEpochOf(this.wasm) : null;
  const currentAngle = Math.atan2(my - s.centerY, mx - s.centerX);
  let deltaDeg = (currentAngle - s.startAngle) * (180 / Math.PI);

  // Ctrl 키: 15° 단위 스냅
  let newAngle = s.origAngle + deltaDeg;
  if (e.ctrlKey) {
    newAngle = Math.round(newAngle / 15) * 15;
  }
  // -360 ~ 360 범위로 정규화
  newAngle = ((newAngle % 360) + 360) % 360;
  if (newAngle > 180) newAngle -= 360;
  const roundedAngle = Math.round(newAngle);
  if (!s.recorder?.started && roundedAngle === s.origAngle) return;

  try {
    const expectedEpoch = s.epoch ?? epochFromLabel(s.beforeLabel) ?? this._gestureOwnerEpoch ?? null;
    s.recorder ??= new ObjectGestureRecorder(
      'rotateObject',
      { sectionIndex: s.ref.sec, paragraphIndex: s.ref.ppi, charOffset: 0 },
      expectedEpoch,
    );
    if (!s.recorder.begin(this.wasm)) {
      InputHandler.prototype.settleOpenPointerGesture.call(this, 'cancel');
      return;
    }
    setObjectProperties.call(this, s.ref, { rotationAngle: roundedAngle });
    try { this.wasm.updateConnectorsInSection(s.ref.sec); } catch { /* ignore */ }
    this.eventBus?.emit('document-changed');
    // 드래그 중에는 핸들 고정 — renderPictureObjectSelection 호출 안 함
  } catch (err) {
    console.warn('[InputHandler] 개체 회전 드래그 실패:', err);
  }
}

/** 회전 드래그 종료: 핸들을 최종 회전 위치로 스냅하고 히스토리 항목 하나로 봉인 */
export function finishPictureRotateDrag(this: any, _e?: MouseEvent): void {
  if (this.pictureRotateState && isActiveGestureEpochStale(this)) {
    InputHandler.prototype.settleOpenPointerGesture.call(this, 'cancel');
    return;
  }
  let command: EditCommand | null = null;
  let hadState = false;
  try {
    if (this.pictureRotateState) {
      hadState = Boolean(this.pictureRotateState.ref);
      command = this.pictureRotateState.recorder?.finish(this.wasm) ?? null;
      if (command) this.history?.recordWithoutExecute(command, this.wasm);
      command = null;
    }
  } finally {
    try {
      command?.discard?.(this.wasm);
      this.pictureRotateState?.recorder?.cancel(this.wasm);
    } finally {
      this._gestureOwnerEpoch = null;
      this.isPictureRotateDragging = false;
      this.pictureRotateState = null;
      if (this.container?.style) this.container.style.cursor = '';
      if (this.dragRafId) {
        cancelAnimationFrame(this.dragRafId);
        this.dragRafId = 0;
      }
    }
  }
  if (hadState) this.renderPictureObjectSelection?.();
}
