// embed 그림 삽입의 이미지 치수 실측(readImageDimensions) 정상/손상-경계 계약을 검증한다.
import assert from 'node:assert/strict';
import test from 'node:test';
import { EmbedInsertPictureCommand } from '../src/engine/embed-commands';
import type { WasmBridge } from '../src/core/wasm-bridge';
import type { DocumentPosition } from '../src/core/types';

const FALLBACK = { width: 960, height: 540 };

/** 명령 실행 후 insertPicture 에 전달된 naturalWidthPx/naturalHeightPx 를 반환한다. */
function measuredDims(bytes: Uint8Array, mime: string): { width: number; height: number } {
  let captured: { width: number; height: number } | null = null;
  const wasm = {
    saveSnapshot: () => 1,
    restoreSnapshot: () => {},
    discardSnapshot: () => {},
    insertPicture: (
      _sec: number, _para: number, _off: number, _data: Uint8Array,
      _w: number, _h: number, naturalWidthPx: number, naturalHeightPx: number,
    ) => {
      captured = { width: naturalWidthPx, height: naturalHeightPx };
      return { ok: true, paraIdx: 0 };
    },
  } as unknown as WasmBridge;
  const position: DocumentPosition = { sectionIndex: 0, paragraphIndex: 0, charOffset: 0 };
  new EmbedInsertPictureCommand(position, { imageRef: { kind: 'binData', bytes, mime } }).execute(wasm);
  assert.ok(captured, 'insertPicture 미호출');
  return captured!;
}

function buf(n: number, fill?: (b: Uint8Array) => void): Uint8Array {
  const b = new Uint8Array(n);
  if (fill) fill(b);
  return b;
}

/** 정상 WebP 컨테이너 공통 골격 — RIFF size/fourcc/chunk size 를 유효하게 채운다. */
function webp(fourcc: string, chunkSize: number, fill: (b: Uint8Array) => void, total = 32): Uint8Array {
  return buf(total, (b) => {
    b.set([0x52, 0x49, 0x46, 0x46], 0); // RIFF
    const riffSize = total - 8;
    b[4] = riffSize & 0xff; b[5] = (riffSize >> 8) & 0xff; b[6] = (riffSize >> 16) & 0xff; b[7] = (riffSize >> 24) & 0xff;
    b.set([0x57, 0x45, 0x42, 0x50], 8); // WEBP
    for (let i = 0; i < 4; i += 1) b[12 + i] = fourcc.charCodeAt(i);
    b[16] = chunkSize & 0xff; b[17] = (chunkSize >> 8) & 0xff; b[18] = (chunkSize >> 16) & 0xff; b[19] = (chunkSize >> 24) & 0xff;
    fill(b);
  });
}

// ── 정상 입력 = 정확 치수 ──

test('PNG IHDR 100x50 실측', () => {
  const png = buf(30, (b) => {
    b.set([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a], 0);
    b.set([0, 0, 0, 13, 0x49, 0x48, 0x44, 0x52], 8);
    b.set([0, 0, 0, 100], 16);
    b.set([0, 0, 0, 50], 20);
  });
  assert.deepEqual(measuredDims(png, 'image/png'), { width: 100, height: 50 });
});

test('JPEG SOF0 100x50 실측 (APP0 세그먼트 스킵 포함)', () => {
  const jpg = buf(40, (b) => {
    b.set([0xff, 0xd8], 0);
    b.set([0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x49, 0x46, 0, 1, 1, 0, 0, 1, 0, 1, 0, 0], 2);
    b.set([0xff, 0xc0, 0x00, 0x11, 0x08, 0x00, 0x32, 0x00, 0x64], 20);
  });
  assert.deepEqual(measuredDims(jpg, 'image/jpeg'), { width: 100, height: 50 });
});

test('GIF89a 100x50 실측', () => {
  const gif = buf(14, (b) => b.set([0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x64, 0x00, 0x32, 0x00], 0));
  assert.deepEqual(measuredDims(gif, 'image/gif'), { width: 100, height: 50 });
});

test('GIF87a 100x50 실측', () => {
  const gif = buf(14, (b) => b.set([0x47, 0x49, 0x46, 0x38, 0x37, 0x61, 0x64, 0x00, 0x32, 0x00], 0));
  assert.deepEqual(measuredDims(gif, 'image/gif'), { width: 100, height: 50 });
});

test('WebP VP8 lossy 100x50 실측', () => {
  const v = webp('VP8 ', 12, (b) => {
    b.set([0x9d, 0x01, 0x2a], 23); // 키프레임 start code
    b[26] = 0x64; b[27] = 0x00; b[28] = 0x32; b[29] = 0x00;
  });
  assert.deepEqual(measuredDims(v, 'image/webp'), { width: 100, height: 50 });
});

test('WebP VP8L lossless 100x50 실측', () => {
  const v = webp('VP8L', 5, (b) => {
    b[20] = 0x2f; b[21] = 0x63; b[22] = 0x40; b[23] = 0x0c; b[24] = 0x00;
  });
  assert.deepEqual(measuredDims(v, 'image/webp'), { width: 100, height: 50 });
});

test('WebP VP8X extended 100x50 실측', () => {
  const v = webp('VP8X', 10, (b) => { b[24] = 99; b[27] = 49; });
  assert.deepEqual(measuredDims(v, 'image/webp'), { width: 100, height: 50 });
});

// ── 손상/미지원 입력 = 960x540 fallback (조작 치수 금지) ──

test('미지원 mime 은 fallback', () => {
  assert.deepEqual(measuredDims(buf(30), 'image/tiff'), FALLBACK);
});

test('극소 버퍼는 fallback', () => {
  assert.deepEqual(measuredDims(buf(2), 'image/png'), FALLBACK);
});

test('잘린 JPEG SOF(segLen<7)는 fallback', () => {
  const jpg = buf(16, (b) => b.set([0xff, 0xd8, 0xff, 0xc0, 0x00, 0x04, 0x08], 0));
  assert.deepEqual(measuredDims(jpg, 'image/jpeg'), FALLBACK);
});

test('버퍼 끝 걸친 JPEG SOF 치수 read 는 fallback', () => {
  // fill 바이트(0xff) 연쇄 뒤 SOF — h/w read 가 버퍼 밖(offset+7>len).
  const jpg = buf(22, (b) => {
    b.fill(0xff);
    b[1] = 0xd8;
    b[18] = 0xc0; b[19] = 0x00; b[20] = 0x11; b[21] = 0x08;
  });
  assert.deepEqual(measuredDims(jpg, 'image/jpeg'), FALLBACK);
});

test('비-87a/89a GIF 시그니처는 fallback', () => {
  const gif = buf(14, (b) => b.set([0x47, 0x49, 0x46, 0x38, 0x38, 0x61, 0x64, 0x00, 0x32, 0x00], 0)); // GIF88a
  assert.deepEqual(measuredDims(gif, 'image/gif'), FALLBACK);
});

test('RIFF 선언 크기가 버퍼 초과인 WebP 는 fallback', () => {
  const v = webp('VP8X', 10, (b) => {
    b[24] = 99; b[27] = 49;
    b[4] = 200; b[5] = 0; b[6] = 0; b[7] = 0; // riffSize=200 > 32-8
  });
  assert.deepEqual(measuredDims(v, 'image/webp'), FALLBACK);
});

test('start code 없는 WebP VP8 은 fallback', () => {
  const v = webp('VP8 ', 12, (b) => {
    b[26] = 0x64; b[28] = 0x32; // 치수만 있고 0x9D012A 없음
  });
  assert.deepEqual(measuredDims(v, 'image/webp'), FALLBACK);
});

test('chunk size 미달 WebP VP8X 는 fallback', () => {
  const v = webp('VP8X', 4, (b) => { b[24] = 99; b[27] = 49; });
  assert.deepEqual(measuredDims(v, 'image/webp'), FALLBACK);
});
