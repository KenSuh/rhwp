// Task #1790 3단계 — 임베드 호스트용 표/그림 spec 삽입 커맨드 (스냅샷 undo 기반).
// GearUp 임베드 런타임에서 검증된 구현을 upstream 으로 이관 — AI 파이프라인이
// 표/그림 spec 을 문서에 history-aware 로 삽입할 때 사용한다.
// 본문 위치와 셀 내부 위치(cellPath) 모두 지원.
import type { WasmBridge } from '@/core/wasm-bridge';
import type { DocumentPosition, CellPathEntry } from '@/core/types';
import type { EditCommand } from '@/engine/command';

/** 셀/표 정렬 값 — ParaProperties.alignment 의 부분집합. */
export type EmbedCellAlign = 'left' | 'center' | 'right' | 'justify';

/** 표 삽입 spec — cells[r][c].text + 선택적 정렬·머리글 서식. */
export interface EmbedTableSpec {
  rows?: number;
  cols?: number;
  cells?: ReadonlyArray<ReadonlyArray<{ text?: string; align?: EmbedCellAlign } | undefined>>;
  /** 표 전체 기본 정렬 (셀별 align 이 우선). */
  align?: EmbedCellAlign;
  /** 첫 행 셀 텍스트를 굵게(머리글) 표시. */
  headerRow?: boolean;
}

/** 그림 삽입 spec — binData 만 지원. */
export interface EmbedPictureSpec {
  imageRef: { kind: 'binData'; bytes: Uint8Array; mime: string } | { kind: string };
  /** "auto" = 원본 폭, { mm } = 지정 폭. */
  width?: 'auto' | { mm: number };
  caption?: string;
  altText?: string;
}

export class EmbedInsertTableCommand implements EditCommand {
  readonly type = 'InsertTableCommand';
  readonly timestamp = Date.now();
  private beforeId: number | null = null;
  private afterId: number | null = null;
  private cursorAfter: DocumentPosition | null = null;
  private readonly position: DocumentPosition;

  constructor(position: DocumentPosition, private readonly spec: EmbedTableSpec) {
    this.position = { ...position };
  }

  execute(wasm: WasmBridge): DocumentPosition {
    if (this.afterId !== null && this.cursorAfter) {
      wasm.restoreSnapshot(this.afterId);
      return { ...this.cursorAfter };
    }
    this.beforeId = wasm.saveSnapshot();
    this.cursorAfter = insertTableSpec(wasm, this.position, this.spec);
    this.afterId = wasm.saveSnapshot();
    return { ...this.cursorAfter };
  }

  undo(wasm: WasmBridge): DocumentPosition {
    if (this.beforeId !== null) {
      wasm.restoreSnapshot(this.beforeId);
    }
    return { ...this.position };
  }

  mergeWith(): EditCommand | null {
    return null;
  }

  discard(wasm: WasmBridge): void {
    if (this.beforeId !== null) {
      wasm.discardSnapshot(this.beforeId);
      this.beforeId = null;
    }
    if (this.afterId !== null) {
      wasm.discardSnapshot(this.afterId);
      this.afterId = null;
    }
  }
}

export class EmbedInsertPictureCommand implements EditCommand {
  readonly type = 'InsertPictureCommand';
  readonly timestamp = Date.now();
  private beforeId: number | null = null;
  private afterId: number | null = null;
  private cursorAfter: DocumentPosition | null = null;
  private readonly position: DocumentPosition;

  constructor(position: DocumentPosition, private readonly spec: EmbedPictureSpec) {
    this.position = { ...position };
  }

  execute(wasm: WasmBridge): DocumentPosition {
    if (this.afterId !== null && this.cursorAfter) {
      wasm.restoreSnapshot(this.afterId);
      return { ...this.cursorAfter };
    }
    this.beforeId = wasm.saveSnapshot();
    this.cursorAfter = insertPictureSpec(wasm, this.position, this.spec);
    this.afterId = wasm.saveSnapshot();
    return { ...this.cursorAfter };
  }

  undo(wasm: WasmBridge): DocumentPosition {
    if (this.beforeId !== null) {
      wasm.restoreSnapshot(this.beforeId);
    }
    return { ...this.position };
  }

  mergeWith(): EditCommand | null {
    return null;
  }

  discard(wasm: WasmBridge): void {
    if (this.beforeId !== null) {
      wasm.discardSnapshot(this.beforeId);
      this.beforeId = null;
    }
    if (this.afterId !== null) {
      wasm.discardSnapshot(this.afterId);
      this.afterId = null;
    }
  }
}

function insertTableSpec(
  wasm: WasmBridge,
  position: DocumentPosition,
  spec: EmbedTableSpec,
): DocumentPosition {
  const rows = Math.max(1, Number(spec?.rows ?? spec?.cells?.length ?? 1));
  const cols = Math.max(1, Number(spec?.cols ?? spec?.cells?.[0]?.length ?? 1));
  const cellPath = documentPositionCellPath(position);
  if (position.parentParaIndex !== undefined && cellPath.length > 0) {
    const result = wasm.createTableInCellByPath(
      position.sectionIndex,
      position.parentParaIndex,
      JSON.stringify(cellPath),
      position.charOffset,
      rows,
      cols,
    );
    if (!result?.ok) return { ...position };

    const cells = spec?.cells ?? [];
    for (let r = 0; r < rows; r += 1) {
      for (let c = 0; c < cols; c += 1) {
        const cell = cells[r]?.[c];
        const text = String(cell?.text ?? '').trim();
        const insertedPath = [
          ...cellPath,
          { controlIndex: result.controlIdx ?? 0, cellIndex: r * cols + c, cellParaIndex: 0 },
        ];
        const insertedPathJson = JSON.stringify(insertedPath);
        if (text) {
          wasm.insertTextInCellByPath(
            position.sectionIndex,
            position.parentParaIndex,
            insertedPathJson,
            0,
            text,
          );
        }
        const align = cell?.align ?? spec.align;
        if (align) {
          wasm.applyParaFormatInCellByPath(
            position.sectionIndex,
            position.parentParaIndex,
            insertedPathJson,
            JSON.stringify({ alignment: align }),
          );
        }
        if (spec.headerRow && r === 0 && text) {
          wasm.applyCharFormatInCellByPath(
            position.sectionIndex,
            position.parentParaIndex,
            insertedPathJson,
            0,
            text.length,
            JSON.stringify({ bold: true }),
          );
        }
      }
    }

    const firstInsertedCellPath = [
      ...cellPath,
      { controlIndex: result.controlIdx ?? 0, cellIndex: 0, cellParaIndex: 0 },
    ];

    return {
      ...position,
      cellPath: firstInsertedCellPath,
      cellIndex: cellPath[0]?.cellIndex ?? position.cellIndex,
      cellParaIndex: cellPath[0]?.cellParaIndex ?? position.cellParaIndex,
      charOffset: 0,
    };
  }

  const result = wasm.createTable(
    position.sectionIndex,
    position.paragraphIndex,
    position.charOffset,
    rows,
    cols,
  );
  if (!result?.ok) return { ...position };

  const paraIdx = result.paraIdx;
  const controlIdx = result.controlIdx ?? 0;
  const cells = spec?.cells ?? [];
  for (let r = 0; r < rows; r += 1) {
    for (let c = 0; c < cols; c += 1) {
      const cell = cells[r]?.[c];
      const text = String(cell?.text ?? '').trim();
      const cellIndex = r * cols + c;
      if (text) {
        wasm.insertTextInCell(
          position.sectionIndex,
          paraIdx,
          controlIdx,
          cellIndex,
          0,
          0,
          text,
        );
      }
      const align = cell?.align ?? spec.align;
      if (align) {
        wasm.applyParaFormatInCell(
          position.sectionIndex,
          paraIdx,
          controlIdx,
          cellIndex,
          0,
          JSON.stringify({ alignment: align }),
        );
      }
      if (spec.headerRow && r === 0 && text) {
        wasm.applyCharFormatInCell(
          position.sectionIndex,
          paraIdx,
          controlIdx,
          cellIndex,
          0,
          0,
          text.length,
          JSON.stringify({ bold: true }),
        );
      }
    }
  }

  return {
    sectionIndex: position.sectionIndex,
    paragraphIndex: paraIdx + 1,
    charOffset: 0,
  };
}

function insertPictureSpec(
  wasm: WasmBridge,
  position: DocumentPosition,
  spec: EmbedPictureSpec,
): DocumentPosition {
  if (spec?.imageRef?.kind !== 'binData') {
    throw new Error('EmbedInsertPictureCommand는 binData 이미지 삽입만 지원합니다.');
  }
  const { bytes, mime } = spec.imageRef as { kind: 'binData'; bytes: Uint8Array; mime: string };
  const dimensions = readImageDimensions(bytes, mime);
  const widthHwp = resolvePictureWidthHwp(spec.width, dimensions.width);
  const ratio = dimensions.height > 0 && dimensions.width > 0
    ? dimensions.height / dimensions.width
    : 9 / 16;
  const heightHwp = Math.max(1, Math.round(widthHwp * ratio));
  const ext = imageMimeToExtension(mime);
  const cellPath = documentPositionCellPath(position);
  if (position.parentParaIndex !== undefined && cellPath.length > 0) {
    const result = wasm.insertPictureInCellByPath(
      position.sectionIndex,
      position.parentParaIndex,
      JSON.stringify(cellPath),
      position.charOffset,
      bytes,
      widthHwp,
      heightHwp,
      dimensions.width,
      dimensions.height,
      ext,
      spec.altText || spec.caption || 'AI 생성 이미지',
    );
    if (!result?.ok) return { ...position };
    return {
      ...position,
      charOffset: result.charOffset ?? position.charOffset + 1,
    };
  }

  const result = wasm.insertPicture(
    position.sectionIndex,
    position.paragraphIndex,
    position.charOffset,
    bytes,
    widthHwp,
    heightHwp,
    dimensions.width,
    dimensions.height,
    ext,
    spec.altText || spec.caption || '',
  );
  if (!result?.ok) return { ...position };
  return {
    sectionIndex: position.sectionIndex,
    paragraphIndex: result.paraIdx + 1,
    charOffset: 0,
  };
}

function documentPositionCellPath(position: DocumentPosition): CellPathEntry[] {
  if (Array.isArray(position?.cellPath) && position.cellPath.length > 0) {
    return position.cellPath.map((entry) => ({
      controlIndex: Number(entry.controlIndex ?? 0),
      cellIndex: Number(entry.cellIndex ?? 0),
      cellParaIndex: Number(entry.cellParaIndex ?? 0),
    }));
  }
  if (
    position?.parentParaIndex !== undefined &&
    position?.controlIndex !== undefined &&
    position?.cellIndex !== undefined
  ) {
    return [{
      controlIndex: Number(position.controlIndex),
      cellIndex: Number(position.cellIndex),
      cellParaIndex: Number(position.cellParaIndex ?? 0),
    }];
  }
  return [];
}

function resolvePictureWidthHwp(
  width: EmbedPictureSpec['width'],
  naturalWidthPx: number,
): number {
  if (width && typeof width === 'object' && Number.isFinite(width.mm)) {
    return Math.max(1, Math.round(width.mm * (7200 / 25.4)));
  }
  return Math.max(1, Math.round((naturalWidthPx || 960) * 75));
}

function imageMimeToExtension(mime: string): string {
  switch (mime) {
    case 'image/jpeg':
      return 'jpg';
    case 'image/webp':
      return 'webp';
    case 'image/gif':
      return 'gif';
    case 'image/png':
    default:
      return 'png';
  }
}

function readImageDimensions(bytes: Uint8Array, mime: string): { width: number; height: number } {
  const fallback = { width: 960, height: 540 };
  if (!bytes || bytes.byteLength < 4) return fallback;
  switch (mime) {
    case 'image/png':
      return readPngDimensions(bytes) ?? fallback;
    case 'image/jpeg':
      return readJpegDimensions(bytes) ?? fallback;
    case 'image/gif':
      return readGifDimensions(bytes) ?? fallback;
    case 'image/webp':
      return readWebpDimensions(bytes) ?? fallback;
    default:
      return fallback;
  }
}

/** PNG — IHDR 청크의 width/height(32bit BE, offset 16/20). */
function readPngDimensions(bytes: Uint8Array): { width: number; height: number } | null {
  if (bytes.byteLength < 24) return null;
  const width = readUint32be(bytes, 16);
  const height = readUint32be(bytes, 20);
  return width > 0 && height > 0 ? { width, height } : null;
}

/** JPEG — SOFn(C0..CF, DHT/JPG/DAC 제외) 프레임 헤더에서 height/width(16bit BE). */
function readJpegDimensions(bytes: Uint8Array): { width: number; height: number } | null {
  if (bytes[0] !== 0xff || bytes[1] !== 0xd8) return null; // SOI
  const len = bytes.byteLength;
  let offset = 2;
  while (offset + 9 < len) {
    if (bytes[offset] !== 0xff) {
      offset += 1;
      continue;
    }
    let marker = bytes[offset + 1] ?? 0;
    while (marker === 0xff && offset + 1 < len) {
      offset += 1; // 마커 앞 fill 바이트(0xff) 스킵
      marker = bytes[offset + 1] ?? 0;
    }
    offset += 2;
    // 길이 없는 standalone 마커: TEM(01), RSTn(D0..D7), EOI(D9)
    if (marker === 0xd9 || marker === 0x01 || (marker >= 0xd0 && marker <= 0xd7)) continue;
    const segLen = readUint16be(bytes, offset);
    const isSof = marker >= 0xc0 && marker <= 0xcf
      && marker !== 0xc4 && marker !== 0xc8 && marker !== 0xcc;
    if (isSof) {
      // SOF 세그먼트 = len(2)+precision(1)+h(2)+w(2) 최소 7바이트. 잘린 SOF는 fallback.
      if (segLen < 7 || offset + 7 > len) return null;
      const height = readUint16be(bytes, offset + 3);
      const width = readUint16be(bytes, offset + 5);
      return width > 0 && height > 0 ? { width, height } : null;
    }
    if (segLen < 2) return null;
    offset += segLen;
  }
  return null;
}

/** GIF — 논리 화면 기술자의 width/height(16bit LE, offset 6/8). */
function readGifDimensions(bytes: Uint8Array): { width: number; height: number } | null {
  if (bytes.byteLength < 10) return null;
  // "GIF87a" 또는 "GIF89a" 정확 매칭
  if (bytes[0] !== 0x47 || bytes[1] !== 0x49 || bytes[2] !== 0x46 || bytes[3] !== 0x38) return null;
  if ((bytes[4] !== 0x37 && bytes[4] !== 0x39) || bytes[5] !== 0x61) return null;
  const width = readUint16le(bytes, 6);
  const height = readUint16le(bytes, 8);
  return width > 0 && height > 0 ? { width, height } : null;
}

/** WebP — VP8(lossy)/VP8L(lossless)/VP8X(extended) 세 컨테이너의 치수. */
function readWebpDimensions(bytes: Uint8Array): { width: number; height: number } | null {
  if (bytes.byteLength < 30) return null;
  // "RIFF" .... "WEBP"
  if (bytes[0] !== 0x52 || bytes[1] !== 0x49 || bytes[2] !== 0x46 || bytes[3] !== 0x46) return null;
  if (bytes[8] !== 0x57 || bytes[9] !== 0x45 || bytes[10] !== 0x42 || bytes[11] !== 0x50) return null;
  // RIFF 선언 크기('WEBP' 4바이트 이상)가 실제 버퍼를 넘으면 잘린 파일 → fallback.
  const riffSize = readUint32le(bytes, 4);
  if (riffSize < 4 || riffSize + 8 > bytes.byteLength) return null;
  const fourcc = String.fromCharCode(bytes[12] ?? 0, bytes[13] ?? 0, bytes[14] ?? 0, bytes[15] ?? 0);
  const chunkSize = readUint32le(bytes, 16);
  if (fourcc === 'VP8 ') {
    // 키프레임 start code(0x9D012A) + 치수(26-29)까지 최소 10바이트 chunk.
    if (chunkSize < 10) return null;
    if (bytes[23] !== 0x9d || bytes[24] !== 0x01 || bytes[25] !== 0x2a) return null;
    const width = readUint16le(bytes, 26) & 0x3fff;
    const height = readUint16le(bytes, 28) & 0x3fff;
    return width > 0 && height > 0 ? { width, height } : null;
  }
  if (fourcc === 'VP8L') {
    // 시그니처(1)+치수 비트필드(4) 최소 5바이트 chunk.
    if (chunkSize < 5) return null;
    if (bytes[20] !== 0x2f) return null; // 시그니처
    const b0 = bytes[21] ?? 0;
    const b1 = bytes[22] ?? 0;
    const b2 = bytes[23] ?? 0;
    const b3 = bytes[24] ?? 0;
    const width = 1 + (((b1 & 0x3f) << 8) | b0);
    const height = 1 + (((b3 & 0x0f) << 10) | (b2 << 2) | ((b1 & 0xc0) >> 6));
    return width > 0 && height > 0 ? { width, height } : null;
  }
  if (fourcc === 'VP8X') {
    // flags(4)+width-1(3)+height-1(3) 고정 10바이트 chunk.
    if (chunkSize < 10) return null;
    const width = 1 + readUint24le(bytes, 24);
    const height = 1 + readUint24le(bytes, 27);
    return width > 0 && height > 0 ? { width, height } : null;
  }
  return null;
}

function readUint16be(bytes: Uint8Array, offset: number): number {
  return ((bytes[offset] ?? 0) << 8) | (bytes[offset + 1] ?? 0);
}

function readUint16le(bytes: Uint8Array, offset: number): number {
  return (bytes[offset] ?? 0) | ((bytes[offset + 1] ?? 0) << 8);
}

function readUint24le(bytes: Uint8Array, offset: number): number {
  return (bytes[offset] ?? 0) | ((bytes[offset + 1] ?? 0) << 8) | ((bytes[offset + 2] ?? 0) << 16);
}

function readUint32le(bytes: Uint8Array, offset: number): number {
  return (
    ((bytes[offset] ?? 0) |
      ((bytes[offset + 1] ?? 0) << 8) |
      ((bytes[offset + 2] ?? 0) << 16)) +
    (bytes[offset + 3] ?? 0) * 0x1000000
  );
}

function readUint32be(bytes: Uint8Array, offset: number): number {
  return (
    ((bytes[offset] ?? 0) << 24) |
    ((bytes[offset + 1] ?? 0) << 16) |
    ((bytes[offset + 2] ?? 0) << 8) |
    (bytes[offset + 3] ?? 0)
  );
}
