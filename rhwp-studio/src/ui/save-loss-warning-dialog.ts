/**
 * 저장 시 내용 손실 경고 대화상자 (save-time hard warning).
 *
 * HWPX serializer 가 emit 하지 못해 저장 시 사라지는 컨트롤(누름틀/수식/책갈피 등)이 있으면
 * 저장 직전에 차단형(blocking) 모달을 띄운다. 일반 `confirm-dialog` 와 달리 **안전한 쪽
 * (취소)이 기본/포커스/Enter/Escape** 이고, 손실을 감수하는 '그래도 저장'은 명시 클릭으로만
 * 동작한다(데이터 손실 방지 — silent loss 를 loud 로).
 */
import type { SaveWarningReport } from '@/core/wasm-bridge';

/** 손실 컨트롤 kind(머신 문자열) → 한국어 라벨. wasm `LossyKind::as_str` 와 1:1. */
const LOSSY_KIND_LABELS: Record<string, string> = {
  Field: '누름틀',
  Bookmark: '책갈피',
  Equation: '수식',
  AutoNumber: '자동 번호',
  NewNumber: '새 번호',
  Footnote: '각주',
  Endnote: '미주',
  Hyperlink: '하이퍼링크',
  Ruby: '덧말',
  CharOverlap: '글자 겹침',
  HiddenComment: '숨은 설명',
  ColumnDef: '다단',
  Header: '머리말',
  Footer: '꼬리말',
  PageHide: '감추기',
  PageNumberPos: '쪽 번호 위치',
  Shape: '그리기 개체',
  TextBox: '글상자',
  Caption: '캡션',
  SectionSettings: '구역 설정',
  Unknown: '알 수 없는 컨트롤',
};

function labelFor(kind: string): string {
  return LOSSY_KIND_LABELS[kind] ?? `알 수 없는 컨트롤(${kind})`;
}

/** 종류별 집계를 "• 누름틀 3개" 형태의 줄 목록으로 만든다(개수 내림차순, 동률은 라벨순). */
function summaryLines(report: SaveWarningReport): string {
  const entries = Object.entries(report.summary);
  entries.sort((a, b) => b[1] - a[1] || labelFor(a[0]).localeCompare(labelFor(b[0]), 'ko'));
  return entries.map(([kind, n]) => `• ${labelFor(kind)} ${n}개`).join('\n');
}

/**
 * 손실 경고 모달을 표시한다. **'그래도 저장'을 명시적으로 클릭했을 때만 `true`** 를 반환하고,
 * 취소/Escape/오버레이/닫기(×)는 모두 `false`(저장 보류). 호출자는 `true` 일 때만 persist 한다.
 */
export function showSaveLossWarning(report: SaveWarningReport): Promise<boolean> {
  return new Promise((resolve) => {
    let settled = false;
    const settle = (proceed: boolean) => {
      if (settled) return;
      settled = true;
      document.removeEventListener('keydown', onKey, true);
      overlay.remove();
      resolve(proceed);
    };

    const overlay = document.createElement('div');
    overlay.className = 'modal-overlay';

    const dialog = document.createElement('div');
    dialog.className = 'dialog-wrap';
    dialog.style.width = '420px';

    // 타이틀 바
    const titleBar = document.createElement('div');
    titleBar.className = 'dialog-title';
    titleBar.textContent = '저장 시 일부 내용이 손실됩니다';
    const closeBtn = document.createElement('button');
    closeBtn.className = 'dialog-close';
    closeBtn.textContent = '×'; // ×
    closeBtn.addEventListener('click', () => settle(false));
    titleBar.appendChild(closeBtn);
    dialog.appendChild(titleBar);

    // 본문
    const body = document.createElement('div');
    body.className = 'dialog-body';
    body.style.padding = '16px 20px';
    body.style.lineHeight = '1.6';

    const intro = document.createElement('div');
    intro.textContent = '다음 내용은 현재 에디터가 HWPX로 저장하지 못해 사라집니다:';
    intro.style.marginBottom = '8px';
    body.appendChild(intro);

    const list = document.createElement('div');
    list.textContent = summaryLines(report);
    list.style.whiteSpace = 'pre-line';
    list.style.fontWeight = '600';
    list.style.margin = '0 0 10px 4px';
    body.appendChild(list);

    const subtext = document.createElement('div');
    subtext.textContent =
      '‘그래도 저장’을 누르면 위 내용이 빠진 채로 저장됩니다. 내용을 보존하려면 ‘취소’를 누르세요.';
    subtext.style.fontSize = '12px';
    subtext.style.color = '#666';
    body.appendChild(subtext);

    dialog.appendChild(body);

    // 하단 버튼 — 안전한 '취소'가 기본/포커스, 파괴적 '그래도 저장'은 명시 클릭만.
    const footer = document.createElement('div');
    footer.className = 'dialog-footer';

    const proceedBtn = document.createElement('button');
    proceedBtn.className = 'dialog-btn';
    proceedBtn.textContent = '그래도 저장';
    proceedBtn.style.color = '#c0392b'; // 파괴적 동작 강조
    proceedBtn.addEventListener('click', () => settle(true));

    const cancelBtn = document.createElement('button');
    // dialog-btn-primary = 기본/포커스 대상(취소를 기본값으로 둠 = fail-safe).
    cancelBtn.className = 'dialog-btn dialog-btn-primary';
    cancelBtn.textContent = '취소';
    cancelBtn.addEventListener('click', () => settle(false));

    footer.appendChild(proceedBtn);
    footer.appendChild(cancelBtn);
    dialog.appendChild(footer);

    overlay.appendChild(dialog);
    overlay.addEventListener('click', (e) => {
      if (e.target === overlay) settle(false);
    });

    // 키보드: Escape/Enter 모두 안전한 '취소'로. '그래도 저장'은 키보드로 트리거되지 않는다.
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape' || e.key === 'Enter') {
        e.stopPropagation();
        e.preventDefault();
        settle(false);
      } else {
        // 모달 외부(편집 영역)로 키 전파 차단.
        e.stopPropagation();
        e.preventDefault();
      }
    };
    document.addEventListener('keydown', onKey, true);

    document.body.appendChild(overlay);
    cancelBtn.focus();
  });
}
