// Task #1790 3단계 — 임베드 호스트용 표/그림 spec 삽입 커맨드 (스냅샷 undo 기반).
// GearUp 임베드 런타임에서 검증된 구현을 upstream 으로 이관 — AI 파이프라인이
// 표/그림 spec 을 문서에 history-aware 로 삽입할 때 사용한다.
// 본문 위치와 셀 내부 위치(cellPath) 모두 지원.
import type { WasmBridge } from '@/core/wasm-bridge';
import type { DocumentPosition, CellPathEntry } from '@/core/types';
import type { EditCommand } from '@/engine/command';

/** 표 삽입 spec — cells[r][c].text 만 사용 (서식 확장은 후속 과제). */
export interface EmbedTableSpec {
  rows?: number;
  cols?: number;
  cells?: ReadonlyArray<ReadonlyArray<{ text?: string } | undefined>>;
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
        const text = String(cells[r]?.[c]?.text ?? '').trim();
        if (!text) continue;
        const insertedPath = [
          ...cellPath,
          { controlIndex: result.controlIdx ?? 0, cellIndex: r * cols + c, cellParaIndex: 0 },
        ];
        wasm.insertTextInCellByPath(
          position.sectionIndex,
          position.parentParaIndex,
          JSON.stringify(insertedPath),
          0,
          text,
        );
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
      const text = String(cells[r]?.[c]?.text ?? '').trim();
      if (!text) continue;
      wasm.insertTextInCell(
        position.sectionIndex,
        paraIdx,
        controlIdx,
        r * cols + c,
        0,
        0,
        text,
      );
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
  if (mime === 'image/png' && bytes?.byteLength >= 24) {
    return {
      width: readUint32be(bytes, 16) || 960,
      height: readUint32be(bytes, 20) || 540,
    };
  }
  return { width: 960, height: 540 };
}

function readUint32be(bytes: Uint8Array, offset: number): number {
  return (
    ((bytes[offset] ?? 0) << 24) |
    ((bytes[offset + 1] ?? 0) << 16) |
    ((bytes[offset + 2] ?? 0) << 8) |
    (bytes[offset + 3] ?? 0)
  );
}
