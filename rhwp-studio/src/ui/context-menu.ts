import type { CommandDispatcher } from '@/command/dispatcher';
import type { CommandRegistry } from '@/command/registry';

type ContextPreviewVisual =
  | 'table'
  | 'timeline'
  | 'compare'
  | 'bar'
  | 'line'
  | 'donut'
  | 'flow'
  | 'org'
  | 'impact'
  | 'scene'
  | 'concept'
  | 'beforeAfter';

/** 컨텍스트 메뉴 항목 정의 */
export interface ContextMenuItem {
  type: 'command' | 'separator' | 'heading';
  commandId?: string;
  label?: string;
  description?: string;
  previewTitle?: string;
  samples?: ReadonlyArray<{
    title: string;
    description: string;
    visual?: ContextPreviewVisual;
  }>;
  previewKind?: 'table' | 'chart' | 'diagram' | 'image';
}

/**
 * 우클릭 컨텍스트 메뉴
 *
 * - show()로 화면 좌표에 메뉴 표시
 * - CommandDispatcher 연동: canExecute 체크로 비활성 항목 표시
 * - ESC / 외부 클릭으로 닫기
 */
export class ContextMenu {
  private el: HTMLDivElement | null = null;
  private previewEl: HTMLDivElement | null = null;
  private previewHideTimer: number | null = null;
  private previewSourceRow: HTMLDivElement | null = null;
  private activePreviewCommandId: string | null = null;
  private escHandler: ((e: KeyboardEvent) => void) | null = null;
  private outsideHandler: ((e: MouseEvent) => void) | null = null;

  constructor(
    private dispatcher: CommandDispatcher,
    private registry: CommandRegistry,
  ) {}

  /** clientX/Y에 메뉴를 표시한다 */
  show(x: number, y: number, items: ContextMenuItem[]): void {
    this.hide();

    const menu = document.createElement('div');
    menu.className = 'context-menu';

    for (const item of items) {
      if (item.type === 'separator') {
        const sep = document.createElement('div');
        sep.className = 'md-sep';
        menu.appendChild(sep);
        continue;
      }

      if (item.type === 'heading') {
        const heading = document.createElement('div');
        heading.className = 'md-heading';
        const label = document.createElement('strong');
        label.textContent = item.label ?? '';
        heading.appendChild(label);
        if (item.description) {
          const desc = document.createElement('span');
          desc.textContent = item.description;
          heading.appendChild(desc);
        }
        menu.appendChild(heading);
        continue;
      }

      const cmdId = item.commandId!;
      const def = this.registry.get(cmdId);
      if (!def) continue;

      const row = document.createElement('div');
      row.className = 'md-item';
      if (item.description) row.classList.add('has-description');
      if (item.samples?.length) row.classList.add('has-preview');
      row.dataset.cmd = cmdId;

      // canExecute 체크
      if (!this.dispatcher.isEnabled(cmdId)) {
        row.classList.add('disabled');
      }

      // 레이블
      if (item.description) {
        const labelBlock = document.createElement('span');
        labelBlock.className = 'md-label-block';
        const label = document.createElement('strong');
        label.textContent = item.label ?? def.label;
        const desc = document.createElement('span');
        desc.textContent = item.description;
        labelBlock.append(label, desc);
        row.appendChild(labelBlock);
      } else {
        const labelSpan = document.createTextNode(item.label ?? def.label);
        row.appendChild(labelSpan);
      }

      // 단축키 표시
      if (def.shortcutLabel) {
        const shortcut = document.createElement('span');
        shortcut.className = 'md-shortcut';
        shortcut.textContent = def.shortcutLabel;
        row.appendChild(shortcut);
      } else if (item.samples?.length) {
        const marker = document.createElement('span');
        marker.className = 'md-preview-mark';
        marker.textContent = '›';
        row.appendChild(marker);
      }

      if (item.samples?.length) {
        const openPreview = () => this.showPreview(menu, row, item, cmdId);
        const deferPreviewClose = () => this.schedulePreviewHide();
        row.addEventListener('mouseenter', openPreview);
        row.addEventListener('pointerenter', openPreview);
        row.addEventListener('mouseleave', deferPreviewClose);
        row.addEventListener('pointerleave', deferPreviewClose);
      } else {
        const closePreview = () => this.hidePreview();
        row.addEventListener('mouseenter', closePreview);
        row.addEventListener('pointerenter', closePreview);
      }

      // 클릭 핸들러
      row.addEventListener('click', (e) => {
        e.stopPropagation();
        if (row.classList.contains('disabled')) return;
        if (item.samples?.length) {
          this.showPreview(menu, row, item, cmdId);
          return;
        }
        this.dispatcher.dispatch(cmdId);
        this.hide();
      });

      menu.appendChild(row);
    }

    document.body.appendChild(menu);
    this.el = menu;

    // 화면 경계 보정
    const rect = menu.getBoundingClientRect();
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    if (x + rect.width > vw) x = vw - rect.width - 2;
    if (y + rect.height > vh) y = vh - rect.height - 2;
    if (x < 0) x = 0;
    if (y < 0) y = 0;
    menu.style.left = `${x}px`;
    menu.style.top = `${y}px`;

    // ESC 닫기
    this.escHandler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        this.hide();
      }
    };
    document.addEventListener('keydown', this.escHandler, true);

    // 외부 클릭 닫기 (다음 이벤트 루프에서 등록)
    requestAnimationFrame(() => {
      this.outsideHandler = (e: MouseEvent) => {
        const target = e.target as Node;
        if (
          this.el &&
          !this.el.contains(target) &&
          !this.previewEl?.contains(target)
        ) {
          this.hide();
        }
      };
      document.addEventListener('mousedown', this.outsideHandler, true);
    });
  }

  /** 메뉴를 닫는다 */
  hide(): void {
    this.hidePreview();
    if (this.escHandler) {
      document.removeEventListener('keydown', this.escHandler, true);
      this.escHandler = null;
    }
    if (this.outsideHandler) {
      document.removeEventListener('mousedown', this.outsideHandler, true);
      this.outsideHandler = null;
    }
    this.el?.remove();
    this.el = null;
  }

  private showPreview(
    menu: HTMLDivElement,
    row: HTMLDivElement,
    item: ContextMenuItem,
    commandId: string,
  ): void {
    if (!item.samples?.length) return;
    this.cancelPreviewHide();
    if (this.activePreviewCommandId === commandId && this.previewEl) {
      this.setPreviewSourceRow(row);
      return;
    }
    this.hidePreview();

    const panel = document.createElement('div');
    panel.className = 'context-menu-preview';
    panel.addEventListener('mouseenter', () => this.cancelPreviewHide());
    panel.addEventListener('pointerenter', () => this.cancelPreviewHide());
    panel.addEventListener('mouseleave', () => this.schedulePreviewHide());
    panel.addEventListener('pointerleave', () => this.schedulePreviewHide());

    const title = document.createElement('strong');
    title.textContent = item.previewTitle ?? item.label ?? '샘플';
    panel.appendChild(title);

    const list = document.createElement('div');
    list.className = 'context-preview-list';
    item.samples.slice(0, 3).forEach((sample, index) => {
      const sampleEl = document.createElement('button');
      sampleEl.type = 'button';
      sampleEl.className = 'context-preview-sample';
      sampleEl.addEventListener('click', (e) => {
        e.preventDefault();
        e.stopPropagation();
        this.dispatcher.dispatch(commandId, { sampleIndex: index });
        this.hide();
      });
      sampleEl.appendChild(this.createSampleVisual(sample.visual, item.previewKind));
      const sampleTitle = document.createElement('b');
      sampleTitle.textContent = sample.title;
      const sampleDesc = document.createElement('span');
      sampleDesc.textContent = sample.description;
      sampleEl.append(sampleTitle, sampleDesc);
      list.appendChild(sampleEl);
    });
    panel.appendChild(list);

    document.body.appendChild(panel);
    this.previewEl = panel;
    this.activePreviewCommandId = commandId;
    this.setPreviewSourceRow(row);

    const menuRect = menu.getBoundingClientRect();
    const rowRect = row.getBoundingClientRect();
    const previewRect = panel.getBoundingClientRect();
    const gap = 6;
    let placement: 'right' | 'left' = 'right';
    let left = menuRect.right + gap;
    if (left + previewRect.width > window.innerWidth - gap) {
      left = menuRect.left - previewRect.width - gap;
      placement = 'left';
    }
    let top = rowRect.top;
    if (top + previewRect.height > window.innerHeight - gap) {
      top = window.innerHeight - previewRect.height - gap;
    }
    panel.dataset.placement = placement;
    panel.style.left = `${Math.max(gap, left)}px`;
    panel.style.top = `${Math.max(gap, top)}px`;
  }

  private schedulePreviewHide(delayMs = 180): void {
    this.cancelPreviewHide();
    this.previewHideTimer = window.setTimeout(() => {
      this.previewHideTimer = null;
      this.hidePreview();
    }, delayMs);
  }

  private cancelPreviewHide(): void {
    if (this.previewHideTimer !== null) {
      window.clearTimeout(this.previewHideTimer);
      this.previewHideTimer = null;
    }
  }

  private setPreviewSourceRow(row: HTMLDivElement): void {
    if (this.previewSourceRow === row) return;
    this.previewSourceRow?.classList.remove('is-preview-active');
    this.previewSourceRow = row;
    this.previewSourceRow.classList.add('is-preview-active');
  }

  private hidePreview(): void {
    this.cancelPreviewHide();
    this.previewSourceRow?.classList.remove('is-preview-active');
    this.previewSourceRow = null;
    this.activePreviewCommandId = null;
    this.previewEl?.remove();
    this.previewEl = null;
  }

  private createSampleVisual(
    visual: ContextPreviewVisual | undefined,
    kind: ContextMenuItem['previewKind'] | undefined,
  ): HTMLSpanElement {
    const shape = visual ?? kind ?? 'table';
    const el = document.createElement('span');
    el.className = `context-preview-visual is-${shape} has-svg`;
    el.innerHTML = sampleVisualSvg(shape);
    return el;
  }

  /** 메뉴가 열려있는가? */
  get isOpen(): boolean {
    return this.el !== null;
  }

  dispose(): void {
    this.hide();
  }
}

function sampleVisualSvg(shape: ContextPreviewVisual | ContextMenuItem['previewKind']): string {
  switch (shape) {
    case 'bar':
      return '<svg viewBox="0 0 94 58" aria-hidden="true"><rect width="94" height="58" rx="6" fill="#f8fafc"/><path d="M16 44H82" stroke="#cbd5e1" stroke-width="1"/><rect x="20" y="28" width="8" height="16" rx="2" fill="#16a34a"/><rect x="32" y="20" width="8" height="24" rx="2" fill="#22c55e"/><rect x="44" y="13" width="8" height="31" rx="2" fill="#15803d"/><rect x="56" y="25" width="8" height="19" rx="2" fill="#60a5fa"/><rect x="68" y="17" width="8" height="27" rx="2" fill="#f59e0b"/></svg>';
    case 'line':
      return '<svg viewBox="0 0 94 58" aria-hidden="true"><rect width="94" height="58" rx="6" fill="#f8fafc"/><path d="M14 44H82" stroke="#cbd5e1"/><path d="M17 39L31 35L45 27L59 30L76 17" fill="none" stroke="#16a34a" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/><g fill="#fff" stroke="#16a34a" stroke-width="2"><circle cx="17" cy="39" r="3"/><circle cx="31" cy="35" r="3"/><circle cx="45" cy="27" r="3"/><circle cx="59" cy="30" r="3"/><circle cx="76" cy="17" r="3"/></g></svg>';
    case 'donut':
      return '<svg viewBox="0 0 94 58" aria-hidden="true"><rect width="94" height="58" rx="6" fill="#f8fafc"/><circle cx="33" cy="29" r="17" fill="none" stroke="#e2e8f0" stroke-width="10"/><circle cx="33" cy="29" r="17" fill="none" stroke="#16a34a" stroke-width="10" stroke-dasharray="46 107" transform="rotate(-90 33 29)"/><circle cx="33" cy="29" r="17" fill="none" stroke="#60a5fa" stroke-width="10" stroke-dasharray="30 107" stroke-dashoffset="-46" transform="rotate(-90 33 29)"/><circle cx="33" cy="29" r="7" fill="#f8fafc"/><rect x="58" y="18" width="20" height="4" rx="2" fill="#16a34a"/><rect x="58" y="28" width="16" height="4" rx="2" fill="#60a5fa"/><rect x="58" y="38" width="12" height="4" rx="2" fill="#f59e0b"/></svg>';
    case 'timeline':
    case 'flow':
      return '<svg viewBox="0 0 94 58" aria-hidden="true"><rect width="94" height="58" rx="6" fill="#f8fafc"/><path d="M18 30H76" stroke="#86efac" stroke-width="3" stroke-linecap="round"/><g fill="#16a34a"><circle cx="18" cy="30" r="7"/><circle cx="37" cy="30" r="7"/><circle cx="56" cy="30" r="7"/><circle cx="75" cy="30" r="7"/></g><path d="M73 25L79 30L73 35" fill="none" stroke="#15803d" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>';
    case 'org':
      return '<svg viewBox="0 0 94 58" aria-hidden="true"><rect width="94" height="58" rx="6" fill="#f8fafc"/><rect x="33" y="8" width="28" height="12" rx="3" fill="#16a34a"/><path d="M47 20V29M22 29H72" stroke="#86efac" stroke-width="2" stroke-linecap="round"/><rect x="10" y="33" width="24" height="13" rx="3" fill="#dcfce7" stroke="#86efac"/><rect x="35" y="33" width="24" height="13" rx="3" fill="#dcfce7" stroke="#86efac"/><rect x="60" y="33" width="24" height="13" rx="3" fill="#dcfce7" stroke="#86efac"/></svg>';
    case 'impact':
      return '<svg viewBox="0 0 94 58" aria-hidden="true"><rect width="94" height="58" rx="6" fill="#f8fafc"/><rect x="8" y="16" width="20" height="26" rx="4" fill="#fee2e2"/><rect x="37" y="12" width="20" height="34" rx="4" fill="#dbeafe"/><rect x="66" y="9" width="20" height="38" rx="4" fill="#dcfce7"/><path d="M29 29H36M58 29H65" stroke="#64748b" stroke-width="2" stroke-linecap="round"/><path d="M34 25L38 29L34 33M63 25L67 29L63 33" fill="none" stroke="#64748b" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>';
    case 'scene':
    case 'concept':
      return '<svg viewBox="0 0 94 58" aria-hidden="true"><rect width="94" height="58" rx="6" fill="#e0f2fe"/><rect x="8" y="34" width="78" height="14" rx="4" fill="#bbf7d0"/><rect x="17" y="23" width="30" height="18" rx="3" fill="#334155"/><rect x="52" y="18" width="22" height="23" rx="4" fill="#16a34a"/><circle cx="72" cy="14" r="7" fill="#f59e0b"/><path d="M23 28H41M57 24H69M57 30H69" stroke="#fff" stroke-width="2" stroke-linecap="round"/></svg>';
    case 'beforeAfter':
      return '<svg viewBox="0 0 94 58" aria-hidden="true"><rect width="94" height="58" rx="6" fill="#f8fafc"/><rect x="8" y="12" width="32" height="34" rx="4" fill="#e2e8f0"/><rect x="54" y="12" width="32" height="34" rx="4" fill="#dcfce7"/><path d="M43 29H51" stroke="#16a34a" stroke-width="2" stroke-linecap="round"/><path d="M48 25L52 29L48 33" fill="none" stroke="#16a34a" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/><path d="M15 35L23 27L31 34" stroke="#94a3b8" stroke-width="2" fill="none"/><path d="M61 36L68 27L78 34" stroke="#16a34a" stroke-width="2" fill="none"/></svg>';
    case 'compare':
      return '<svg viewBox="0 0 94 58" aria-hidden="true"><rect width="94" height="58" rx="6" fill="#f8fafc"/><rect x="9" y="10" width="34" height="38" rx="4" fill="#fff" stroke="#cbd5e1"/><rect x="51" y="10" width="34" height="38" rx="4" fill="#fff" stroke="#bbf7d0"/><rect x="14" y="18" width="20" height="4" rx="2" fill="#94a3b8"/><rect x="14" y="28" width="15" height="4" rx="2" fill="#94a3b8"/><rect x="56" y="18" width="20" height="4" rx="2" fill="#16a34a"/><rect x="56" y="28" width="15" height="4" rx="2" fill="#16a34a"/></svg>';
    case 'table':
    default:
      return '<svg viewBox="0 0 94 58" aria-hidden="true"><rect width="94" height="58" rx="6" fill="#f8fafc"/><rect x="9" y="10" width="76" height="38" rx="4" fill="#fff" stroke="#dbe3ec"/><rect x="9" y="10" width="76" height="10" rx="4" fill="#bbf7d0"/><path d="M9 29H85M9 39H85M34 10V48M60 10V48" stroke="#e2e8f0"/><rect x="15" y="26" width="13" height="3" rx="1.5" fill="#94a3b8"/><rect x="40" y="36" width="13" height="3" rx="1.5" fill="#16a34a"/><rect x="66" y="26" width="13" height="3" rx="1.5" fill="#60a5fa"/></svg>';
  }
}
