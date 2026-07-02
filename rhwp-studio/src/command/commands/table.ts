import type { DocumentPosition } from '@/core/types';
import type { CommandDef, CommandServices, EditorContext } from '../types';
import { TableCellPropsDialog } from '@/ui/table-cell-props-dialog';
import { TableCreateDialog } from '@/ui/table-create-dialog';
import { CellSplitDialog } from '@/ui/cell-split-dialog';
import { CellBorderBgDialog } from '@/ui/cell-border-bg-dialog';
import { FormulaDialog } from '@/ui/formula-dialog';

const inTable = (ctx: EditorContext) => ctx.inTable;

function nestedCellPathJson(pos: { parentParaIndex?: number; cellPath?: unknown[] }): string | null {
  return pos.parentParaIndex !== undefined && Array.isArray(pos.cellPath) && pos.cellPath.length > 1
    ? JSON.stringify(pos.cellPath)
    : null;
}

function insertionCellPath(pos: {
  parentParaIndex?: number;
  controlIndex?: number;
  cellIndex?: number;
  cellParaIndex?: number;
  cellPath?: unknown[];
}): Array<{ controlIndex: number; cellIndex: number; cellParaIndex: number }> {
  if (Array.isArray(pos.cellPath) && pos.cellPath.length > 0) {
    return pos.cellPath as Array<{ controlIndex: number; cellIndex: number; cellParaIndex: number }>;
  }
  if (
    pos.parentParaIndex !== undefined &&
    pos.controlIndex !== undefined &&
    pos.cellIndex !== undefined &&
    pos.cellParaIndex !== undefined
  ) {
    return [{
      controlIndex: pos.controlIndex,
      cellIndex: pos.cellIndex,
      cellParaIndex: pos.cellParaIndex,
    }];
  }
  return [];
}

function tableCtxPathJson(ctx: { cellPath?: unknown[] } | null | undefined): string | null {
  return ctx?.cellPath && ctx.cellPath.length > 1 ? JSON.stringify(ctx.cellPath) : null;
}

function targetCellIndex(pos: { cellIndex?: number; cellPath?: unknown[] }): number | undefined {
  if (Array.isArray(pos.cellPath) && pos.cellPath.length > 0) {
    const last = pos.cellPath[pos.cellPath.length - 1] as { cellIndex?: number } | undefined;
    if (typeof last?.cellIndex === 'number') return last.cellIndex;
  }
  return pos.cellIndex;
}

function cursorAfterTableDelete(pos: DocumentPosition): DocumentPosition {
  const cellPath = pos.cellPath;
  if (pos.parentParaIndex !== undefined && Array.isArray(cellPath) && cellPath.length > 1) {
    const parentPath = cellPath.slice(0, -1);
    const lastParent = parentPath[parentPath.length - 1];
    return {
      sectionIndex: pos.sectionIndex,
      paragraphIndex: lastParent.cellParaIndex,
      charOffset: 0,
      parentParaIndex: pos.parentParaIndex,
      controlIndex: parentPath[0].controlIndex,
      cellIndex: lastParent.cellIndex,
      cellParaIndex: lastParent.cellParaIndex,
      cellPath: parentPath,
    };
  }
  return {
    sectionIndex: pos.sectionIndex,
    paragraphIndex: pos.parentParaIndex ?? pos.paragraphIndex,
    charOffset: 0,
  };
}

function runTableSnapshot(
  services: CommandServices,
  operationType: string,
  operation: (wasm: CommandServices['wasm']) => DocumentPosition | void,
): void {
  const ih = services.getInputHandler();
  if (!ih) return;
  ih.executeOperation({
    kind: 'snapshot',
    operationType,
    operation: (wasm) => operation(wasm) ?? ih.getCursorPosition(),
  });
}

function getCurrentCellInfo(services: any, pos: any) {
  const pathJson = nestedCellPathJson(pos);
  if (pathJson) {
    return services.wasm.getCellInfoByPath(pos.sectionIndex, pos.parentParaIndex, pathJson);
  }
  return services.wasm.getCellInfo(pos.sectionIndex, pos.parentParaIndex, pos.controlIndex, pos.cellIndex);
}

function stub(id: string, label: string, icon?: string, shortcut?: string): CommandDef {
  return {
    id,
    label,
    icon,
    shortcutLabel: shortcut,
    canExecute: inTable,
    execute() { /* TODO: 후속 타스크에서 구현 */ },
  };
}

export const tableCommands: CommandDef[] = [
  { id: 'table:create', label: '표 만들기', icon: 'icon-table',
    canExecute: (ctx) => ctx.hasDocument && ctx.isEditable,
    execute(services, params) {
      const ih = services.getInputHandler();
      if (!ih) return;
      const pos = ih.getCursorPosition();
      const dialog = new TableCreateDialog();
      dialog.onApply = (rows, cols) => {
        try {
          const cellPath = insertionCellPath(pos);
          runTableSnapshot(services, 'createTable', (wasm) => {
            const result = pos.parentParaIndex !== undefined && cellPath.length > 0
              ? wasm.createTableInCellByPath(
                  pos.sectionIndex,
                  pos.parentParaIndex,
                  JSON.stringify(cellPath),
                  pos.charOffset,
                  rows,
                  cols,
                )
              : wasm.createTable(
                  pos.sectionIndex, pos.paragraphIndex, pos.charOffset,
                  rows, cols,
                );
            if (result.ok) {
              if (pos.parentParaIndex !== undefined && cellPath.length > 0) {
                const createdPath = [
                  ...cellPath,
                  { controlIndex: result.controlIdx, cellIndex: 0, cellParaIndex: 0 },
                ];
                return {
                  sectionIndex: pos.sectionIndex,
                  paragraphIndex: 0,
                  charOffset: 0,
                  parentParaIndex: pos.parentParaIndex,
                  controlIndex: createdPath[0].controlIndex,
                  cellIndex: 0,
                  cellParaIndex: 0,
                  cellPath: createdPath,
                };
              }
              return {
                sectionIndex: pos.sectionIndex,
                paragraphIndex: 0,
                charOffset: 0,
                parentParaIndex: result.paraIdx,
                controlIndex: 0,
                cellIndex: 0,
                cellParaIndex: 0,
              };
            }
            return pos;
          });
        } catch (e) {
          console.error('표 만들기 실패:', e);
        }
      };
      dialog.show(params?.anchorEl as HTMLElement | undefined);
    },
  },
  {
    id: 'table:cell-props',
    label: '표/셀 속성',
    canExecute: inTable,
    execute(services) {
      const ih = services.getInputHandler();
      if (!ih) return;
      const pos = ih.getCursorPosition();
      if (pos.parentParaIndex === undefined || pos.controlIndex === undefined || pos.cellIndex === undefined) return;
      const cellIdx = targetCellIndex(pos);
      if (cellIdx === undefined) return;
      const tableCtx = { sec: pos.sectionIndex, ppi: pos.parentParaIndex, ci: pos.controlIndex, cellPath: pos.cellPath };
      const ih2 = services.getInputHandler();
      const mode = ih2?.isInTableObjectSelection() ? 'table' as const : 'cell' as const;
      const dialog = new TableCellPropsDialog(services.wasm, services.eventBus, tableCtx, cellIdx, mode);
      dialog.show();
    },
  },
  {
    id: 'table:border-each',
    label: '각 셀마다 적용(E)...',
    canExecute: inTable,
    execute(services) {
      const ih = services.getInputHandler();
      if (!ih) return;
      const pos = ih.getCursorPosition();
      if (pos.parentParaIndex === undefined || pos.controlIndex === undefined || pos.cellIndex === undefined) return;
      const cellIdx = targetCellIndex(pos);
      if (cellIdx === undefined) return;
      const tableCtx = { sec: pos.sectionIndex, ppi: pos.parentParaIndex, ci: pos.controlIndex, cellPath: pos.cellPath };
      const dialog = new CellBorderBgDialog(services.wasm, services.eventBus, tableCtx, cellIdx, 'each');
      dialog.show();
    },
  },
  {
    id: 'table:border-one',
    label: '하나의 셀처럼 적용(Z)...',
    canExecute: inTable,
    execute(services) {
      const ih = services.getInputHandler();
      if (!ih) return;
      const pos = ih.getCursorPosition();
      if (pos.parentParaIndex === undefined || pos.controlIndex === undefined || pos.cellIndex === undefined) return;
      const cellIdx = targetCellIndex(pos);
      if (cellIdx === undefined) return;
      const tableCtx = { sec: pos.sectionIndex, ppi: pos.parentParaIndex, ci: pos.controlIndex, cellPath: pos.cellPath };
      const dialog = new CellBorderBgDialog(services.wasm, services.eventBus, tableCtx, cellIdx, 'asOne');
      dialog.show();
    },
  },
  {
    id: 'table:insert-row-above',
    label: '위쪽에 줄 추가하기',
    canExecute: inTable,
    execute(services) {
      const ih = services.getInputHandler();
      if (!ih) return;
      const pos = ih.getCursorPosition();
      if (pos.parentParaIndex === undefined || pos.controlIndex === undefined || pos.cellIndex === undefined) return;
      const cellInfo = getCurrentCellInfo(services, pos);
      const pathJson = nestedCellPathJson(pos);
      try {
        runTableSnapshot(services, 'insertTableRowAbove', (wasm) => {
          if (pathJson) {
            wasm.insertTableRowByPath(pos.sectionIndex, pos.parentParaIndex, pathJson, cellInfo.row, false);
          } else {
            wasm.insertTableRow(pos.sectionIndex, pos.parentParaIndex, pos.controlIndex, cellInfo.row, false);
          }
        });
      } catch (e) {
        console.error('줄 추가 실패:', e);
      }
    },
  },
  {
    id: 'table:insert-row-below',
    label: '아래쪽에 줄 추가하기',
    canExecute: inTable,
    execute(services) {
      const ih = services.getInputHandler();
      if (!ih) return;
      const pos = ih.getCursorPosition();
      if (pos.parentParaIndex === undefined || pos.controlIndex === undefined || pos.cellIndex === undefined) return;
      const cellInfo = getCurrentCellInfo(services, pos);
      const pathJson = nestedCellPathJson(pos);
      try {
        runTableSnapshot(services, 'insertTableRowBelow', (wasm) => {
          if (pathJson) {
            wasm.insertTableRowByPath(pos.sectionIndex, pos.parentParaIndex, pathJson, cellInfo.row, true);
          } else {
            wasm.insertTableRow(pos.sectionIndex, pos.parentParaIndex, pos.controlIndex, cellInfo.row, true);
          }
        });
      } catch (e) {
        console.error('줄 추가 실패:', e);
      }
    },
  },
  {
    id: 'table:insert-col-left',
    label: '왼쪽에 칸 추가하기',
    shortcutLabel: 'Alt+Insert',
    canExecute: inTable,
    execute(services) {
      const ih = services.getInputHandler();
      if (!ih) return;
      const pos = ih.getCursorPosition();
      if (pos.parentParaIndex === undefined || pos.controlIndex === undefined || pos.cellIndex === undefined) return;
      const cellInfo = getCurrentCellInfo(services, pos);
      const pathJson = nestedCellPathJson(pos);
      try {
        runTableSnapshot(services, 'insertTableColumnLeft', (wasm) => {
          if (pathJson) {
            wasm.insertTableColumnByPath(pos.sectionIndex, pos.parentParaIndex, pathJson, cellInfo.col, false);
          } else {
            wasm.insertTableColumn(pos.sectionIndex, pos.parentParaIndex, pos.controlIndex, cellInfo.col, false);
          }
        });
      } catch (e) {
        console.error('칸 추가 실패:', e);
      }
    },
  },
  {
    id: 'table:insert-col-right',
    label: '오른쪽에 칸 추가하기',
    canExecute: inTable,
    execute(services) {
      const ih = services.getInputHandler();
      if (!ih) return;
      const pos = ih.getCursorPosition();
      if (pos.parentParaIndex === undefined || pos.controlIndex === undefined || pos.cellIndex === undefined) return;
      const cellInfo = getCurrentCellInfo(services, pos);
      const pathJson = nestedCellPathJson(pos);
      try {
        runTableSnapshot(services, 'insertTableColumnRight', (wasm) => {
          if (pathJson) {
            wasm.insertTableColumnByPath(pos.sectionIndex, pos.parentParaIndex, pathJson, cellInfo.col, true);
          } else {
            wasm.insertTableColumn(pos.sectionIndex, pos.parentParaIndex, pos.controlIndex, cellInfo.col, true);
          }
        });
      } catch (e) {
        console.error('칸 추가 실패:', e);
      }
    },
  },
  {
    id: 'table:delete-row',
    label: '줄 지우기',
    canExecute: inTable,
    execute(services) {
      const ih = services.getInputHandler();
      if (!ih) return;
      const pos = ih.getCursorPosition();
      if (pos.parentParaIndex === undefined || pos.controlIndex === undefined || pos.cellIndex === undefined) return;
      const cellInfo = getCurrentCellInfo(services, pos);
      const pathJson = nestedCellPathJson(pos);
      try {
        runTableSnapshot(services, 'deleteTableRow', (wasm) => {
          if (pathJson) {
            wasm.deleteTableRowByPath(pos.sectionIndex, pos.parentParaIndex, pathJson, cellInfo.row);
          } else {
            wasm.deleteTableRow(pos.sectionIndex, pos.parentParaIndex, pos.controlIndex, cellInfo.row);
          }
        });
      } catch (e) {
        console.error('줄 지우기 실패:', e);
      }
    },
  },
  {
    id: 'table:delete-col',
    label: '칸 지우기',
    shortcutLabel: 'Alt+Delete',
    canExecute: inTable,
    execute(services) {
      const ih = services.getInputHandler();
      if (!ih) return;
      const pos = ih.getCursorPosition();
      if (pos.parentParaIndex === undefined || pos.controlIndex === undefined || pos.cellIndex === undefined) return;
      const cellInfo = getCurrentCellInfo(services, pos);
      const pathJson = nestedCellPathJson(pos);
      try {
        runTableSnapshot(services, 'deleteTableColumn', (wasm) => {
          if (pathJson) {
            wasm.deleteTableColumnByPath(pos.sectionIndex, pos.parentParaIndex, pathJson, cellInfo.col);
          } else {
            wasm.deleteTableColumn(pos.sectionIndex, pos.parentParaIndex, pos.controlIndex, cellInfo.col);
          }
        });
      } catch (e) {
        console.error('칸 지우기 실패:', e);
      }
    },
  },
  {
    id: 'table:cell-split',
    label: '셀 나누기',
    shortcutLabel: 'S',
    canExecute: inTable,
    execute(services) {
      const ih = services.getInputHandler();
      if (!ih) return;
      const pos = ih.getCursorPosition();
      if (pos.parentParaIndex === undefined || pos.controlIndex === undefined || pos.cellIndex === undefined) return;

      // F5 셀 선택 모드: 범위 선택 여부 확인
      const range = ih.getSelectedCellRange?.();
      const tableCtx = ih.getCellTableContext?.();
      const isMultiCell = range && tableCtx &&
        (range.startRow !== range.endRow || range.startCol !== range.endCol);

      const cellInfo = getCurrentCellInfo(services, pos);
      const pathJson = nestedCellPathJson(pos);
      const isMerged = !isMultiCell && (cellInfo.rowSpan > 1 || cellInfo.colSpan > 1);

      const dialog = new CellSplitDialog(isMerged);
      dialog.onApply = (nRows, mCols, equalHeight, mergeFirst) => {
        try {
          runTableSnapshot(services, 'splitTableCell', (wasm) => {
            if (isMultiCell && range && tableCtx) {
              const tablePathJson = tableCtxPathJson(tableCtx);
              // 다중 셀: 범위 내 각 셀을 개별 분할
              if (tablePathJson) {
                wasm.splitTableCellsInRangeByPath(
                  tableCtx.sec, tableCtx.ppi, tablePathJson,
                  range.startRow, range.startCol, range.endRow, range.endCol,
                  nRows, mCols, equalHeight,
                );
              } else {
                wasm.splitTableCellsInRange(
                  tableCtx.sec, tableCtx.ppi, tableCtx.ci,
                  range.startRow, range.startCol, range.endRow, range.endCol,
                  nRows, mCols, equalHeight,
                );
              }
              ih.exitCellSelectionMode?.();
            } else {
              // 단일 셀 분할
              if (pathJson) {
                wasm.splitTableCellIntoByPath(
                  pos.sectionIndex, pos.parentParaIndex!, pathJson,
                  cellInfo.row, cellInfo.col,
                  nRows, mCols, equalHeight, mergeFirst,
                );
              } else {
                wasm.splitTableCellInto(
                  pos.sectionIndex, pos.parentParaIndex!, pos.controlIndex!,
                  cellInfo.row, cellInfo.col,
                  nRows, mCols, equalHeight, mergeFirst,
                );
              }
            }
          });
        } catch (e) {
          console.error('셀 나누기 실패:', e);
        }
      };
      dialog.show();
    },
  },
  {
    id: 'table:cell-merge',
    label: '셀 합치기',
    shortcutLabel: 'M',
    canExecute: (ctx) => ctx.inCellSelectionMode,
    execute(services) {
      const ih = services.getInputHandler();
      if (!ih) return;
      const range = ih.getSelectedCellRange();
      const tableCtx = ih.getCellTableContext();
      if (!range || !tableCtx) return;
      if (range.startRow === range.endRow && range.startCol === range.endCol) return;
      try {
        const pathJson = tableCtxPathJson(tableCtx);
        runTableSnapshot(services, 'mergeTableCells', (wasm) => {
          if (pathJson) {
            wasm.mergeTableCellsByPath(tableCtx.sec, tableCtx.ppi, pathJson, range.startRow, range.startCol, range.endRow, range.endCol);
          } else {
            wasm.mergeTableCells(tableCtx.sec, tableCtx.ppi, tableCtx.ci, range.startRow, range.startCol, range.endRow, range.endCol);
          }
          ih.exitCellSelectionMode();
        });
      } catch (e) {
        console.error('셀 합치기 실패:', e);
      }
    },
  },
  {
    id: 'table:delete',
    label: '표 지우기',
    canExecute: (ctx) => ctx.inTable || ctx.inTableObjectSelection,
    execute(services) {
      const ih = services.getInputHandler();
      if (!ih) return;
      // 표 객체 선택 모드면 선택된 표 참조 사용
      const ref = ih.getSelectedTableRef();
      if (ref) {
        try {
          const cursorAfterDelete = ih.moveOutOfSelectedTable();
          runTableSnapshot(services, 'deleteTable', (wasm) => {
            if (ref.cellPath && ref.cellPath.length > 1) {
              wasm.deleteTableControlByPath(ref.sec, ref.ppi, JSON.stringify(ref.cellPath));
            } else {
              wasm.deleteTableControl(ref.sec, ref.ppi, ref.ci);
            }
            return cursorAfterDelete;
          });
        } catch (e) {
          console.error('표 지우기 실패:', e);
        }
        return;
      }
      // 셀 내부에서 커맨드 실행
      const pos = ih.getCursorPosition();
      if (pos.parentParaIndex === undefined || pos.controlIndex === undefined) return;
      try {
        const pathJson = nestedCellPathJson(pos);
        const cursorAfterDelete = cursorAfterTableDelete(pos);
        runTableSnapshot(services, 'deleteTable', (wasm) => {
          if (pathJson) {
            wasm.deleteTableControlByPath(pos.sectionIndex, pos.parentParaIndex, pathJson);
          } else {
            wasm.deleteTableControl(pos.sectionIndex, pos.parentParaIndex, pos.controlIndex);
          }
          return cursorAfterDelete;
        });
      } catch (e) {
        console.error('표 지우기 실패:', e);
      }
    },
  },
  {
    id: 'table:caption-toggle',
    label: '캡션 넣기',
    canExecute: (ctx) => ctx.inTable || ctx.inTableObjectSelection,
    execute(services) {
      const ih = services.getInputHandler();
      if (!ih) return;
      // 표 참조 획득 (표 객체 선택 또는 셀 내부)
      let sec: number, ppi: number, ci: number;
      const ref = ih.getSelectedTableRef();
      if (ref) {
        sec = ref.sec; ppi = ref.ppi; ci = ref.ci;
      } else {
        const pos = ih.getCursorPosition();
        if (pos.parentParaIndex === undefined || pos.controlIndex === undefined) return;
        sec = pos.sectionIndex; ppi = pos.parentParaIndex; ci = pos.controlIndex;
      }
      // 현재 캡션 상태 조회
      let props: any;
      try { props = services.wasm.getTableProperties(sec, ppi, ci); } catch { return; }
      if (!props) return;
      let charOffset = 0;
      if (!props.hasCaption) {
        try {
          const result: any = services.wasm.setTableProperties(sec, ppi, ci, { hasCaption: true });
          charOffset = result?.captionCharOffset ?? 3;
          services.eventBus.emit('document-changed');
        } catch (e) { console.error('표 캡션 생성 실패:', e); return; }
      } else {
        try {
          const len = services.wasm.getCellParagraphLength(sec, ppi, ci, 65534, 0);
          charOffset = len;
        } catch { charOffset = 0; }
      }
      // 표 내부 편집 모드 종료 후 캡션 편집 진입
      if (ref) {
        ih.exitTableObjectSelection();
      }
      ih.enterTableCaptionEditing(sec, ppi, ci, charOffset);
    },
  },
  {
    id: 'table:cell-height-equal',
    label: '셀 높이를 같게',
    shortcutLabel: 'H',
    canExecute: (ctx) => ctx.inCellSelectionMode,
    execute(services) {
      services.getInputHandler()?.performEqualizeSelectedCellSize('height');
    },
  },
  {
    id: 'table:cell-width-equal',
    label: '셀 너비를 같게',
    shortcutLabel: 'W',
    canExecute: (ctx) => ctx.inCellSelectionMode,
    execute(services) {
      services.getInputHandler()?.performEqualizeSelectedCellSize('width');
    },
  },
  {
    id: 'table:formula',
    label: '계산식(F)...',
    shortcutLabel: 'Ctrl+N,F',
    canExecute: inTable,
    execute(services) {
      const ih = services.getInputHandler();
      if (!ih) return;
      const pos = ih.getCursorPosition();
      if (pos.parentParaIndex === undefined || pos.controlIndex === undefined || pos.cellIndex === undefined) return;
      const cellIdx = targetCellIndex(pos);
      if (cellIdx === undefined) return;
      const dialog = new FormulaDialog(services.wasm, services.eventBus, {
        sec: pos.sectionIndex,
        ppi: pos.parentParaIndex,
        ci: pos.controlIndex,
        cellIndex: cellIdx,
        cellPath: pos.cellPath,
      });
      dialog.show();
    },
  },
  stub('table:block-formula', '블록 계산식'),
  stub('table:block-sum', '블록 합계', undefined, 'Ctrl+Shift+S'),
  stub('table:block-avg', '블록 평균', undefined, 'Ctrl+Shift+A'),
  stub('table:block-product', '블록 곱', undefined, 'Ctrl+Shift+P'),
  stub('table:thousand-sep', '1,000 단위 구분 쉼표'),
  stub('table:decimal-add', '자릿점 넣기'),
  stub('table:decimal-remove', '자릿점 빼기'),
];
