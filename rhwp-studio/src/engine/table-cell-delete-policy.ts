// 선택 셀 오려두기/지우기에서 rhwp식 분기 정책을 고정한다.
import type { CellDeleteChoice } from '@/ui/cell-delete-choice-dialog';

export type CellCutDeletePlan =
  | 'request-choice'
  | 'clear-contents'
  | 'delete-cells'
  | 'cancel';

export function resolveCellCutDeletePlan(options: {
  isFullRowSelection: boolean;
  choice?: CellDeleteChoice;
}): CellCutDeletePlan {
  const { isFullRowSelection, choice } = options;

  if (!isFullRowSelection) return 'clear-contents';
  if (!choice) return 'request-choice';
  if (choice === 'cancel') return 'cancel';
  if (choice === 'delete-cells') return 'delete-cells';
  return 'clear-contents';
}
