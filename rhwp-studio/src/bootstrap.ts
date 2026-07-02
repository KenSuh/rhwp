// Task #1790 — main.ts 의 초기화 로직을 재사용 가능한 bootstrap(rootEl, opts) 로 추출.
// 1단계: 추출 (스탠드얼론 동작 불변 — opts 미지정 시 기존과 완전 동일).
// 2단계: 임베드 opts + 반환 핸들 — 외부 호스트(GearUp iframe 등)가 rhwp 를
//        복제 없이 정본 그대로 소비할 수 있는 공식 표면.
// DOM 접근은 rootEl 스코프 query 로 통일 (동일 id 는 studio-root 내부에 존재).
// document/window 레벨 리스너(전역 단축키·drop 방지·postMessage)는 기존과 동일하게 유지.
import { WasmBridge } from '@/core/wasm-bridge';
import type { DocumentInfo, DocumentPosition } from '@/core/types';
import type { ContextMenuItem } from '@/ui/context-menu';
import { StudioExtensionAPI } from '@/command/extension-api';
import { InsertTextCommand, SplitParagraphCommand, SplitParagraphInCellCommand } from '@/engine/command';
import type { EditCommand } from '@/engine/command';
import {
  EmbedInsertPictureCommand,
  EmbedInsertTableCommand,
  type EmbedPictureSpec,
  type EmbedTableSpec,
} from '@/engine/embed-commands';
import { EventBus } from '@/core/event-bus';
import { CanvasView } from '@/view/canvas-view';
import { InputHandler } from '@/engine/input-handler';
import { Toolbar } from '@/ui/toolbar';
import { MenuBar } from '@/ui/menu-bar';
import { loadWebFonts } from '@/core/font-loader';
import { CommandRegistry } from '@/command/registry';
import { CommandDispatcher } from '@/command/dispatcher';
import type { EditorContext, CommandServices } from '@/command/types';
import { fileCommands } from '@/command/commands/file';
import { editCommands } from '@/command/commands/edit';
import { viewCommands } from '@/command/commands/view';
import { formatCommands } from '@/command/commands/format';
import { insertCommands } from '@/command/commands/insert';
import { tableCommands } from '@/command/commands/table';
import { pageCommands } from '@/command/commands/page';
import { toolCommands } from '@/command/commands/tool';
import { ContextMenu } from '@/ui/context-menu';
import { CommandPalette } from '@/ui/command-palette';
import { showValidationModalIfNeeded } from '@/ui/validation-modal';
import { showToast } from '@/ui/toast';
import { CellSelectionRenderer } from '@/engine/cell-selection-renderer';
import { TableObjectRenderer } from '@/engine/table-object-renderer';
import { TableResizeRenderer } from '@/engine/table-resize-renderer';
import { Ruler } from '@/view/ruler';

/** 임베드 호스트가 주입하는 옵션 — 미지정 시 스탠드얼론 기본 동작. */
export interface BootstrapOpts {
  /** iframe postMessage API(hwpctl-load / rhwp-request) 등록 여부. 기본 true. */
  enablePostMessageApi?: boolean;
  /** 컨텍스트 메뉴 항목 확장 훅 — 호스트가 항목을 추가/재배열. */
  extendContextMenuItems?(input: {
    x: number;
    y: number;
    items: ReadonlyArray<ContextMenuItem>;
    context: EditorContext;
  }): ReadonlyArray<ContextMenuItem>;
  /** 임베드 정책. */
  embed?: {
    /**
     * 줌 하한. zoom ≤ 0.5(GRID_ZOOM_THRESHOLD) 에서 grid(다중 열) 레이아웃로
     * 전환되면 마우스 히트테스트(단일 열 전제)와 어긋나 편집이 깨지므로,
     * 편집 임베드는 0.5 초과 값(예: 0.55)을 지정해 grid 진입을 차단한다.
     * 문서 로드 시 현재 줌이 하한 미만이면 100% 로 정규화.
     */
    minZoom?: number;
  };
}

/** bootstrap 반환 핸들 — 임베드 호스트의 공식 소비 표면. */
export interface BootstrapHandle {
  wasm: WasmBridge;
  eventBus: EventBus;
  extensionAPI: StudioExtensionAPI;
  createInsertTextCommand(pos: DocumentPosition, text: string): EditCommand;
  createSplitParagraphCommand(pos: DocumentPosition): EditCommand;
  /** 3단계 — 표/그림 spec 삽입 (스냅샷 undo·본문/셀 내부 지원). */
  createInsertTableCommand(pos: DocumentPosition, spec: EmbedTableSpec): EditCommand;
  createInsertPictureCommand(pos: DocumentPosition, spec: EmbedPictureSpec): EditCommand;
  /** WASM + 폰트 + InputHandler 초기화 완료 promise. 실패 시 reject. */
  ready: Promise<void>;
  getInputHandler(): InputHandler | null;
  getCanvasView(): CanvasView | null;
  loadFile(file: File): Promise<void>;
  loadBytes(data: Uint8Array, fileName: string, fileHandle?: WasmBridge['currentFileHandle']): Promise<void>;
  createNewDocument(): Promise<void>;
  destroy(): void;
}

export function bootstrap(rootEl: HTMLElement, opts: BootstrapOpts = {}): BootstrapHandle {
  const q = <T extends HTMLElement = HTMLElement>(id: string): T =>
    rootEl.querySelector(`#${id}`) as T;

  const wasm = new WasmBridge();
  const eventBus = new EventBus();

  // E2E 테스트용 전역 노출 (개발 모드 전용)
  if (import.meta.env.DEV) {
    (window as any).__wasm = wasm;
    (window as any).__eventBus = eventBus;
  }
  let canvasView: CanvasView | null = null;
  let inputHandler: InputHandler | null = null;
  let toolbar: Toolbar | null = null;
  let ruler: Ruler | null = null;

  // ─── 커맨드 시스템 ─────────────────────────────
  const registry = new CommandRegistry();

  function getContext(): EditorContext {
    const hasDoc = wasm.pageCount > 0;
    return {
      hasDocument: hasDoc,
      hasSelection: inputHandler?.hasSelection() ?? false,
      inTable: inputHandler?.isInTable() ?? false,
      inCellSelectionMode: inputHandler?.isInCellSelectionMode() ?? false,
      inTableObjectSelection: inputHandler?.isInTableObjectSelection() ?? false,
      inPictureObjectSelection: inputHandler?.isInPictureObjectSelection() ?? false,
      inField: inputHandler?.isInField() ?? false,
      isEditable: true,
      canUndo: inputHandler?.canUndo() ?? false,
      canRedo: inputHandler?.canRedo() ?? false,
      zoom: canvasView?.getViewportManager().getZoom() ?? 1.0,
      showControlCodes: wasm.getShowControlCodes(),
      sourceFormat: hasDoc ? (wasm.getSourceFormat() as 'hwp' | 'hwpx') : undefined,
    };
  }

  const commandServices: CommandServices = {
    eventBus,
    wasm,
    getContext,
    getInputHandler: () => inputHandler,
    getViewportManager: () => canvasView?.getViewportManager() ?? null,
  };

  const dispatcher = new CommandDispatcher(registry, commandServices, eventBus);

  // 모든 내장 커맨드 등록
  registry.registerAll(fileCommands);
  registry.registerAll(editCommands);
  registry.registerAll(viewCommands);
  registry.registerAll(formatCommands);
  registry.registerAll(insertCommands);
  registry.registerAll(tableCommands);
  registry.registerAll(pageCommands);
  registry.registerAll(toolCommands);

  // 상태 바 요소
  const sbMessage = () => q('sb-message');
  const sbPage = () => q('sb-page');
  const sbSection = () => q('sb-section');
  const sbZoomVal = () => q('sb-zoom-val');

  async function initialize(): Promise<void> {
    const msg = sbMessage();
    try {
      msg.textContent = '웹폰트 로딩 중...';
      await loadWebFonts([]);  // CSS @font-face 등록 + CRITICAL 폰트만 로드
      msg.textContent = 'WASM 로딩 중...';
      await wasm.initialize();
      msg.textContent = 'HWP 파일을 선택해주세요.';

      const container = q('scroll-container');
      canvasView = new CanvasView(container, wasm, eventBus);

      // 임베드 줌 하한 — grid 레이아웃(마우스 히트테스트 미지원) 진입 차단.
      const minZoom = opts.embed?.minZoom;
      if (minZoom !== undefined) {
        const vm = canvasView.getViewportManager();
        const rawSetZoom = vm.setZoom.bind(vm);
        vm.setZoom = (zoom: number) => rawSetZoom(Math.max(minZoom, zoom));
      }

      // 눈금자 초기화
      ruler = new Ruler(
        q<HTMLCanvasElement>('h-ruler'),
        q<HTMLCanvasElement>('v-ruler'),
        container,
        eventBus,
        wasm,
        canvasView.getVirtualScroll(),
        canvasView.getViewportManager(),
      );

      inputHandler = new InputHandler(
        container, wasm, eventBus,
        canvasView.getVirtualScroll(),
        canvasView.getViewportManager(),
      );

      toolbar = new Toolbar(q('style-bar'), wasm, eventBus, dispatcher);
      toolbar.setEnabled(false);

      // InputHandler에 커맨드 디스패처 및 컨텍스트 메뉴 주입
      inputHandler.setDispatcher(dispatcher);
      const contextMenu = new ContextMenu(dispatcher, registry);
      // 임베드 훅 — 호스트가 컨텍스트 메뉴 항목을 확장/재배열.
      if (opts.extendContextMenuItems) {
        const extend = opts.extendContextMenuItems;
        const rawShow = contextMenu.show.bind(contextMenu);
        contextMenu.show = (x, y, items) => {
          const extended = extend({ x, y, items, context: getContext() });
          rawShow(x, y, (extended ?? items) as ContextMenuItem[]);
        };
      }
      inputHandler.setContextMenu(contextMenu);
      inputHandler.setCommandPalette(new CommandPalette(registry, dispatcher));
      inputHandler.setCellSelectionRenderer(
        new CellSelectionRenderer(container, canvasView.getVirtualScroll()),
      );
      inputHandler.setTableObjectRenderer(
        new TableObjectRenderer(container, canvasView.getVirtualScroll()),
      );
      inputHandler.setTableResizeRenderer(
        new TableResizeRenderer(container, canvasView.getVirtualScroll()),
      );
      inputHandler.setPictureObjectRenderer(
        new TableObjectRenderer(container, canvasView.getVirtualScroll(), true),
      );

      new MenuBar(q('menu-bar'), eventBus, dispatcher);

      // 툴바 내 data-cmd 버튼 클릭 → 커맨드 디스패치
      rootEl.querySelectorAll('.tb-btn[data-cmd]').forEach(btn => {
        btn.addEventListener('mousedown', (e) => {
          e.preventDefault();
          const cmd = (btn as HTMLElement).dataset.cmd;
          if (cmd) dispatcher.dispatch(cmd, { anchorEl: btn as HTMLElement });
        });
      });

      // 스플릿 버튼 드롭다운 메뉴
      rootEl.querySelectorAll('.tb-split').forEach(split => {
        const arrow = split.querySelector('.tb-split-arrow');
        if (arrow) {
          arrow.addEventListener('mousedown', (e) => {
            e.preventDefault();
            e.stopPropagation();
            // 다른 열린 메뉴 닫기
            rootEl.querySelectorAll('.tb-split.open').forEach(s => {
              if (s !== split) s.classList.remove('open');
            });
            split.classList.toggle('open');
          });
        }
        split.querySelectorAll('.tb-split-item[data-cmd]').forEach(item => {
          item.addEventListener('mousedown', (e) => {
            e.preventDefault();
            split.classList.remove('open');
            const cmd = (item as HTMLElement).dataset.cmd;
            if (cmd) dispatcher.dispatch(cmd, { anchorEl: item as HTMLElement });
          });
        });
      });
      // 외부 클릭 시 스플릿 메뉴 닫기
      document.addEventListener('mousedown', () => {
        rootEl.querySelectorAll('.tb-split.open').forEach(s => s.classList.remove('open'));
      });

      setupFileInput();
      setupZoomControls();
      setupEventListeners();
      setupGlobalShortcuts();
      loadFromUrlParam();

      // E2E 테스트용 전역 노출 (개발 모드 전용)
      if (import.meta.env.DEV) {
        (window as any).__inputHandler = inputHandler;
        (window as any).__canvasView = canvasView;
      }
    } catch (error) {
      msg.textContent = `WASM 초기화 실패: ${error}`;
      console.error('[main] WASM 초기화 실패:', error);
      // 임베드 핸들의 ready 가 실패를 감지할 수 있게 rethrow.
      throw error;
    }
  }

  /**
   * 전역 단축키 핸들러 — InputHandler.active 여부와 무관하게 동작해야 하는 단축키.
   * 예: 문서 미로드 상태에서도 Alt+N(새 문서), Ctrl+O(열기) 등.
   */
  function setupGlobalShortcuts(): void {
    document.addEventListener('keydown', (e) => {
      // input/textarea 등 편집 가능 요소 내부에서는 무시
      const target = e.target as HTMLElement;
      if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) return;
      // InputHandler가 활성 상태이면 자체 처리에 맡김
      if (inputHandler?.isActive()) return;

      const ctrlOrMeta = e.ctrlKey || e.metaKey;

      // Alt+N / Alt+ㅜ → 새 문서 (문서 미로드 상태에서도 동작)
      if (e.altKey && !ctrlOrMeta && !e.shiftKey) {
        if (e.key === 'n' || e.key === 'N' || e.key === 'ㅜ') {
          e.preventDefault();
          dispatcher.dispatch('file:new-doc');
          return;
        }
      }
    }, false);
  }

  function setupFileInput(): void {
    const fileInput = q<HTMLInputElement>('file-input');

    fileInput.addEventListener('change', async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) return;
      const name = file.name.toLowerCase();
      if (!name.endsWith('.hwp') && !name.endsWith('.hwpx')) {
        alert('HWP/HWPX 파일만 지원합니다.');
        return;
      }
      await loadFile(file);
    });

    // 문서 전체에서 브라우저 기본 드롭 동작 방지 (파일 열기/다운로드 방지)
    document.addEventListener('dragover', (e) => e.preventDefault());
    document.addEventListener('drop', (e) => e.preventDefault());

    // 드래그 앤 드롭 지원 (scroll-container 영역)
    const container = q('scroll-container');
    container.addEventListener('dragover', (e) => {
      e.preventDefault();
      container.classList.add('drag-over');
    });
    container.addEventListener('dragleave', () => {
      container.classList.remove('drag-over');
    });
    container.addEventListener('drop', async (e) => {
      e.preventDefault();
      container.classList.remove('drag-over');
      const file = e.dataTransfer?.files[0];
      if (!file) return;
      const dropName = file.name.toLowerCase();
      if (!dropName.endsWith('.hwp') && !dropName.endsWith('.hwpx')) {
        alert('HWP/HWPX 파일만 지원합니다.');
        return;
      }
      await loadFile(file);
    });
  }

  function setupZoomControls(): void {
    if (!canvasView) return;
    const vm = canvasView.getViewportManager();

    q('sb-zoom-in').addEventListener('click', () => {
      vm.setZoom(vm.getZoom() + 0.1);
    });
    q('sb-zoom-out').addEventListener('click', () => {
      vm.setZoom(vm.getZoom() - 0.1);
    });

    // 폭 맞춤: 용지 폭에 맞게 줌 조절
    q('sb-zoom-fit-width').addEventListener('click', () => {
      if (wasm.pageCount === 0) return;
      const container = q('scroll-container');
      const containerWidth = container.clientWidth - 40; // 좌우 여백 제외
      const pageInfo = wasm.getPageInfo(0);
      // pageInfo.width는 이미 px 단위 (96dpi 기준)
      const zoom = containerWidth / pageInfo.width;
      console.log(`[zoom-fit-width] container=${containerWidth} page=${pageInfo.width} zoom=${zoom.toFixed(3)}`);
      vm.setZoom(Math.max(0.1, Math.min(zoom, 4.0)));
    });

    // 쪽 맞춤: 한 페이지 전체가 보이도록 줌 조절
    q('sb-zoom-fit').addEventListener('click', () => {
      if (wasm.pageCount === 0) return;
      const container = q('scroll-container');
      const containerWidth = container.clientWidth - 40;
      const containerHeight = container.clientHeight - 40;
      const pageInfo = wasm.getPageInfo(0);
      // pageInfo.width/height는 이미 px 단위 (96dpi 기준)
      const zoomW = containerWidth / pageInfo.width;
      const zoomH = containerHeight / pageInfo.height;
      console.log(`[zoom-fit-page] containerW=${containerWidth} containerH=${containerHeight} pageW=${pageInfo.width} pageH=${pageInfo.height} zoomW=${zoomW.toFixed(3)} zoomH=${zoomH.toFixed(3)}`);
      vm.setZoom(Math.max(0.1, Math.min(zoomW, zoomH, 4.0)));
    });

    // 모바일: 줌 값 클릭 → 100% 토글
    q('sb-zoom-val').addEventListener('click', () => {
      const currentZoom = vm.getZoom();
      if (Math.abs(currentZoom - 1.0) < 0.05) {
        // 현재 100% → 쪽 맞춤으로 전환
        q('sb-zoom-fit').click();
      } else {
        // 현재 쪽 맞춤/기타 → 100%로 전환
        vm.setZoom(1.0);
      }
    });

    document.addEventListener('keydown', (e) => {
      if (!e.ctrlKey && !e.metaKey) return;
      if (e.key === '=' || e.key === '+') {
        e.preventDefault();
        vm.setZoom(vm.getZoom() + 0.1);
      } else if (e.key === '-') {
        e.preventDefault();
        vm.setZoom(vm.getZoom() - 0.1);
      } else if (e.key === '0') {
        e.preventDefault();
        vm.setZoom(1.0);
      }
    });
  }

  let totalSections = 1;

  function setupEventListeners(): void {
    eventBus.on('current-page-changed', (page, _total) => {
      const pageIdx = page as number;
      sbPage().textContent = `${pageIdx + 1} / ${_total} 쪽`;

      // 구역 정보: 현재 페이지의 sectionIndex로 갱신
      if (wasm.pageCount > 0) {
        try {
          const pageInfo = wasm.getPageInfo(pageIdx);
          sbSection().textContent = `구역: ${pageInfo.sectionIndex + 1} / ${totalSections}`;
        } catch { /* 무시 */ }
      }
    });

    eventBus.on('zoom-level-display', (zoom) => {
      sbZoomVal().textContent = `${Math.round((zoom as number) * 100)}%`;
    });

    // 삽입/수정 모드 토글
    eventBus.on('insert-mode-changed', (insertMode) => {
      q('sb-mode').textContent = (insertMode as boolean) ? '삽입' : '수정';
    });

    // 필드 정보 표시
    const sbField = rootEl.querySelector<HTMLElement>('#sb-field');
    eventBus.on('field-info-changed', (info) => {
      if (!sbField) return;
      const fi = info as { fieldId: number; fieldType: string; guideName?: string } | null;
      if (fi) {
        const label = fi.guideName || `#${fi.fieldId}`;
        sbField.textContent = `[누름틀] ${label}`;
        sbField.style.display = '';
      } else {
        sbField.textContent = '';
        sbField.style.display = 'none';
      }
    });

    // 개체 선택 시 회전/대칭 버튼 그룹 표시/숨김
    const rotateGroup = rootEl.querySelector<HTMLElement>('.tb-rotate-group');
    if (rotateGroup) {
      eventBus.on('picture-object-selection-changed', (selected) => {
        rotateGroup.style.display = (selected as boolean) ? '' : 'none';
      });
    }

    // 머리말/꼬리말 편집 모드 시 도구상자 전환 + 본문 dimming
    const hfGroup = rootEl.querySelector<HTMLElement>('.tb-headerfooter-group');
    const hfLabel = hfGroup?.querySelector<HTMLElement>('.tb-hf-label') ?? null;
    const defaultTbGroups = rootEl.querySelectorAll('#icon-toolbar > .tb-group:not(.tb-headerfooter-group):not(.tb-rotate-group), #icon-toolbar > .tb-sep');
    const scrollContainer = q('scroll-container');
    const styleBar = q('style-bar');

    eventBus.on('headerFooterModeChanged', (mode) => {
      const isActive = (mode as string) !== 'none';
      // 도구상자 전환
      if (hfGroup) {
        hfGroup.style.display = isActive ? '' : 'none';
      }
      if (hfLabel) {
        hfLabel.textContent = (mode as string) === 'header' ? '머리말' : (mode as string) === 'footer' ? '꼬리말' : '';
      }
      defaultTbGroups.forEach((el) => {
        (el as HTMLElement).style.display = isActive ? 'none' : '';
      });
      // 서식 도구 모음은 머리말/꼬리말 편집 시에도 유지 (문단/글자 모양 설정 필요)
      // 본문 dimming
      if (scrollContainer) {
        if (isActive) {
          scrollContainer.classList.add('hf-editing');
        } else {
          scrollContainer.classList.remove('hf-editing');
        }
      }
      void styleBar;
    });
  }

  /** 문서 초기화 공통 시퀀스 (loadFile, createNewDocument 양쪽에서 사용) */
  async function initializeDocument(docInfo: DocumentInfo, displayName: string): Promise<void> {
    const msg = sbMessage();
    try {
      console.log('[initDoc] 1. 폰트 로딩 시작');
      if (docInfo.fontsUsed?.length) {
        await loadWebFonts(docInfo.fontsUsed, (loaded, total) => {
          msg.textContent = `폰트 로딩 중... (${loaded}/${total})`;
        });
      }
      console.log('[initDoc] 2. 폰트 로딩 완료');
      msg.textContent = displayName;
      totalSections = docInfo.sectionCount ?? 1;
      sbSection().textContent = `구역: 1 / ${totalSections}`;
      console.log('[initDoc] 3. inputHandler deactivate');
      inputHandler?.deactivate();
      console.log('[initDoc] 4. canvasView loadDocument');
      canvasView?.loadDocument();
      // 임베드 줌 정규화 — 이전 상태(모바일 폭맞춤 등)로 하한 미만이면 100% 로.
      {
        const minZoom = opts.embed?.minZoom;
        const vm = canvasView?.getViewportManager();
        if (minZoom !== undefined && vm && vm.getZoom() < minZoom) vm.setZoom(1.0);
      }
      console.log('[initDoc] 5. toolbar setEnabled');
      toolbar?.setEnabled(true);
      console.log('[initDoc] 6. toolbar initStyleDropdown');
      toolbar?.initStyleDropdown();
      console.log('[initDoc] 7. inputHandler activateWithCaretPosition');
      inputHandler?.activateWithCaretPosition();
      console.log('[initDoc] 8. 완료');

      // #177: HWPX 비표준 lineseg 감지 → 경고 있으면 모달로 사용자 선택 요청
      try {
        const report = wasm.getValidationWarnings();
        console.log(`[validation] ${report.count} warnings`, report.summary);
        if (report.count > 0) {
          const choice = await showValidationModalIfNeeded(report);
          console.log(`[validation] user choice: ${choice}`);
          if (choice === 'auto-fix') {
            const result = wasm.autoFixValidationWarnings();
            console.log('[validation] auto-fix result', result);
            // 렌더 재계산
            canvasView?.loadDocument();
            msg.textContent = result.remainingWarnings === 0
              ? `${displayName} (비표준 lineseg ${result.fixedWarnings}건 자동 보정됨)`
              : `${displayName} (비표준 ${result.fixedWarnings}건 보정, ${result.remainingWarnings}건 확인 필요)`;
          }
        }
      } catch (e) {
        console.warn('[validation] 감지/보정 실패 (치명적이지 않음):', e);
      }
    } catch (error) {
      console.error('[initDoc] 오류:', error);
      if (window.innerWidth < 768) alert(`초기화 오류: ${error}`);
    }
  }

  async function loadFile(file: File): Promise<void> {
    const msg = sbMessage();
    try {
      msg.textContent = '파일 로딩 중...';
      const startTime = performance.now();
      const data = new Uint8Array(await file.arrayBuffer());
      await loadBytes(data, file.name, null, startTime);
    } catch (error) {
      const errMsg = `파일 로드 실패: ${error}`;
      msg.textContent = errMsg;
      console.error('[main] 파일 로드 실패:', error);
      // 모바일에서 상태 메시지가 숨겨질 수 있으므로 alert으로도 표시
      if (window.innerWidth < 768) alert(errMsg);
    }
  }

  async function loadBytes(
    data: Uint8Array,
    fileName: string,
    fileHandle: typeof wasm.currentFileHandle,
    startTime = performance.now(),
  ): Promise<void> {
    const docInfo = wasm.loadDocument(data, fileName);
    wasm.currentFileHandle = fileHandle;
    const elapsed = performance.now() - startTime;
    // initializeDocument 안에서 #177 validation 모달이 표시될 수 있음.
    // HWPX 토스트는 모달과의 이벤트 충돌을 피하기 위해 모달 닫힌 후 표시.
    await initializeDocument(docInfo, `${fileName} — ${docInfo.pageCount}페이지 (${elapsed.toFixed(1)}ms)`);
    notifyHwpxBetaIfNeeded();
  }

  /**
   * #196: HWPX 출처 문서 로드 시 베타 안내.
   * - 우상단 토스트 1회
   * - 상태 표시줄 메시지
   */
  function notifyHwpxBetaIfNeeded(): void {
    if (wasm.getSourceFormat() !== 'hwpx') return;

    showToast({
      message: 'HWPX 편집/저장은 베타 단계입니다.\n저장 후 재열기 검증을 권장합니다.',
      durationMs: 0, // 자동 페이드 없음 — 사용자가 확인 버튼으로 닫음
      confirmLabel: '확인',
    });

    const sb = sbMessage();
    if (sb) sb.textContent = 'HWPX 편집/저장 베타 — 저장 가능';
  }

  async function createNewDocument(): Promise<void> {
    const msg = sbMessage();
    try {
      msg.textContent = '새 문서 생성 중...';
      const docInfo = wasm.createNewDocument();
      await initializeDocument(docInfo, `새 문서.hwp — ${docInfo.pageCount}페이지`);
    } catch (error) {
      msg.textContent = `새 문서 생성 실패: ${error}`;
      console.error('[main] 새 문서 생성 실패:', error);
    }
  }

  // 커맨드에서 새 문서 생성 호출
  eventBus.on('create-new-document', () => { createNewDocument(); });
  eventBus.on('open-document-bytes', async (payload) => {
    const data = payload as {
      bytes: Uint8Array;
      fileName: string;
      fileHandle: typeof wasm.currentFileHandle;
    };
    await loadBytes(data.bytes, data.fileName, data.fileHandle);
  });

  // 수식 더블클릭 → 수식 편집 대화상자
  eventBus.on('equation-edit-request', () => {
    dispatcher.dispatch('insert:equation-edit');
  });

  /**
   * URL 파라미터(?url=)로 전달된 HWP 파일을 자동 로드한다.
   * Chrome 확장 프로그램에서 뷰어 탭을 열 때 사용.
   */
  async function loadFromUrlParam(): Promise<void> {
    const params = new URLSearchParams(window.location.search);
    const fileUrl = params.get('url');
    if (!fileUrl) return;

    const fileName = params.get('filename') || fileUrl.split('/').pop()?.split('?')[0] || 'document.hwp';
    const msg = sbMessage();

    try {
      msg.textContent = '파일 로딩 중...';
      console.log(`[loadFromUrlParam] ${fileUrl}`);

      let response: Response;

      // Chrome 확장 환경: Service Worker를 통한 CORS 우회 fetch
      if (typeof chrome !== 'undefined' && chrome.runtime?.sendMessage) {
        try {
          response = await fetch(fileUrl);
        } catch {
          // 직접 fetch 실패 시 Service Worker 프록시
          const result = await chrome.runtime.sendMessage({ type: 'fetch-file', url: fileUrl });
          if (result.error) throw new Error(result.error);
          const data = new Uint8Array(result.data);
          const docInfo = wasm.loadDocument(data, fileName);
          await initializeDocument(docInfo, `${fileName} — ${docInfo.pageCount}페이지`);
          return;
        }
      } else {
        response = await fetch(fileUrl);
      }

      if (!response.ok) throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      const buffer = await response.arrayBuffer();
      const data = new Uint8Array(buffer);
      const docInfo = wasm.loadDocument(data, fileName);
      await initializeDocument(docInfo, `${fileName} — ${docInfo.pageCount}페이지`);
    } catch (error) {
      const errMsg = `파일 로드 실패: ${error}`;
      msg.textContent = errMsg;
      console.error('[loadFromUrlParam]', error);
    }
  }

  const ready = initialize();
  // 스탠드얼론(핸들 미사용)에서 초기화 실패가 unhandled rejection 이 되지 않게 —
  // 실패 메시지는 initialize 내부에서 상태 표시줄에 이미 노출됨.
  ready.catch(() => { /* handled in initialize */ });

  // ── iframe 연동 API (postMessage) ──
  // 부모 페이지에서 postMessage로 에디터를 제어할 수 있다.
  // 요청: { type: 'rhwp-request', id, method, params }
  // 응답: { type: 'rhwp-response', id, result?, error? }
  if (opts.enablePostMessageApi !== false) window.addEventListener('message', async (e) => {
    const msg = e.data;
    if (!msg || typeof msg !== 'object') return;

    // 기존 hwpctl-load 호환
    if (msg.type === 'hwpctl-load' && msg.data) {
      try {
        const bytes = new Uint8Array(msg.data);
        const docInfo = wasm.loadDocument(bytes, msg.fileName || 'document.hwp');
        await initializeDocument(docInfo, `${msg.fileName || 'document'} — ${docInfo.pageCount}페이지`);
        e.source?.postMessage({ type: 'rhwp-response', id: msg.id, result: { pageCount: docInfo.pageCount } }, { targetOrigin: '*' });
      } catch (err: any) {
        e.source?.postMessage({ type: 'rhwp-response', id: msg.id, error: err.message || String(err) }, { targetOrigin: '*' });
      }
      return;
    }

    // rhwp-request: 범용 API
    if (msg.type !== 'rhwp-request' || !msg.method) return;
    const { id, method, params } = msg;
    const reply = (result?: any, error?: string) => {
      e.source?.postMessage({ type: 'rhwp-response', id, result, error }, { targetOrigin: '*' });
    };

    try {
      switch (method) {
        case 'loadFile': {
          const bytes = new Uint8Array(params.data);
          const docInfo = wasm.loadDocument(bytes, params.fileName || 'document.hwp');
          await initializeDocument(docInfo, `${params.fileName || 'document'} — ${docInfo.pageCount}페이지`);
          reply({ pageCount: docInfo.pageCount });
          break;
        }
        case 'pageCount':
          reply(wasm.pageCount);
          break;
        case 'getPageSvg':
          reply(wasm.renderPageSvg(params.page ?? 0));
          break;
        case 'ready':
          reply(true);
          break;
        default:
          reply(undefined, `Unknown method: ${method}`);
      }
    } catch (err: any) {
      reply(undefined, err.message || String(err));
    }
  });

  void ruler;

  // ── 임베드 핸들 ──
  const extensionAPI = new StudioExtensionAPI(registry, dispatcher, q('menu-bar'));
  return {
    wasm,
    eventBus,
    extensionAPI,
    createInsertTextCommand: (pos, text) => new InsertTextCommand(pos, text),
    createSplitParagraphCommand: (pos) =>
      pos.parentParaIndex !== undefined
        ? new SplitParagraphInCellCommand(pos)
        : new SplitParagraphCommand(pos),
    createInsertTableCommand: (pos, spec) => new EmbedInsertTableCommand(pos, spec),
    createInsertPictureCommand: (pos, spec) => new EmbedInsertPictureCommand(pos, spec),
    ready,
    getInputHandler: () => inputHandler,
    getCanvasView: () => canvasView,
    loadFile,
    loadBytes: (data, fileName, fileHandle = null) => loadBytes(data, fileName, fileHandle),
    createNewDocument,
    destroy: () => {
      inputHandler?.deactivate();
      eventBus.removeAll();
      rootEl.innerHTML = '';
    },
  };
}
