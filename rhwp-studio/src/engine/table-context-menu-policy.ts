// 표 컨텍스트 메뉴 구성을 테스트 가능한 순수 정책으로 고정한다.
import type { ContextMenuItem } from '@/ui/context-menu';

export function buildTableContextMenuItems(options: {
  inCellSelection: boolean;
  hasSelection: boolean;
}): ContextMenuItem[] {
  const { inCellSelection, hasSelection } = options;
  const primaryItems: ContextMenuItem[] = [
    { type: 'command', commandId: 'edit:cut' },
    { type: 'command', commandId: 'edit:copy' },
    { type: 'command', commandId: 'edit:paste' },
  ];
  if (inCellSelection || hasSelection) {
    primaryItems.push({ type: 'command', commandId: 'edit:delete', label: '지우기' });
  }

  const cellSelectionItems: ContextMenuItem[] = inCellSelection
    ? [
        { type: 'separator' },
        { type: 'command', commandId: 'table:cell-height-equal' },
        { type: 'command', commandId: 'table:cell-width-equal' },
        { type: 'command', commandId: 'table:cell-merge' },
        { type: 'command', commandId: 'table:cell-split' },
      ]
    : [];

  return [
    ...primaryItems,
    ...cellSelectionItems,
    { type: 'separator' },
    { type: 'command', commandId: 'table:cell-props', label: '셀 속성...' },
    { type: 'separator' },
    { type: 'command', commandId: 'table:insert-row-above' },
    { type: 'command', commandId: 'table:insert-row-below' },
    { type: 'command', commandId: 'table:insert-col-left' },
    { type: 'command', commandId: 'table:insert-col-right' },
    { type: 'separator' },
    { type: 'command', commandId: 'table:delete-row' },
    { type: 'command', commandId: 'table:delete-col' },
    { type: 'separator' },
    ...(!inCellSelection ? [{ type: 'command' as const, commandId: 'table:cell-split' }] : []),
    { type: 'separator' },
    { type: 'command', commandId: 'table:border-each', label: '셀 테두리/배경 - 각 셀마다 적용(E)...' },
    { type: 'command', commandId: 'table:border-one', label: '셀 테두리/배경 - 하나의 셀처럼 적용(Z)...' },
    { type: 'separator' },
    { type: 'command', commandId: 'table:caption-toggle', label: '캡션 넣기(A)' },
    { type: 'separator' },
    { type: 'command', commandId: 'table:formula', label: '계산식(F)...' },
    { type: 'separator' },
    { type: 'command', commandId: 'table:delete' },
  ];
}
