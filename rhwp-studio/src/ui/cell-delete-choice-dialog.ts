/** 선택 셀 삭제/오려두기 동작에서 셀 모양 유지 여부를 묻는 대화상자. */
export type CellDeleteChoice = 'keep-shape' | 'delete-cells' | 'cancel';
export type CellPasteChoice = 'overwrite' | 'cancel';

export function showCellDeleteChoiceDialog(action: 'cut' | 'delete'): Promise<CellDeleteChoice> {
  return new Promise((resolve) => {
    let resolved = false;
    const finish = (choice: CellDeleteChoice) => {
      if (resolved) return;
      resolved = true;
      document.removeEventListener('keydown', onKeyDown, true);
      overlay.remove();
      resolve(choice);
    };

    const overlay = document.createElement('div');
    overlay.className = 'modal-overlay';

    const dialog = document.createElement('div');
    dialog.className = 'dialog-wrap cell-delete-choice-dialog';
    dialog.style.width = '360px';

    const title = document.createElement('div');
    title.className = 'dialog-title';
    title.textContent = action === 'cut' ? '셀 오려 두기' : '셀 지우기';

    const closeBtn = document.createElement('button');
    closeBtn.className = 'dialog-close';
    closeBtn.textContent = '\u00D7';
    closeBtn.addEventListener('click', () => finish('cancel'));
    title.appendChild(closeBtn);

    const body = document.createElement('div');
    body.className = 'dialog-body';
    body.style.padding = '18px 20px';
    body.style.lineHeight = '1.55';
    body.textContent = action === 'cut'
      ? '선택된 셀들을 오려냅니다. 내용만 지우고 셀 모양은 남겨 둘까요?'
      : '선택된 셀들을 지웁니다. 내용만 지우고 셀 모양은 남겨 둘까요?';

    const footer = document.createElement('div');
    footer.className = 'dialog-footer';

    const keepBtn = document.createElement('button');
    keepBtn.className = 'dialog-btn dialog-btn-primary';
    keepBtn.textContent = '남김';
    keepBtn.addEventListener('click', () => finish('keep-shape'));

    const deleteBtn = document.createElement('button');
    deleteBtn.className = 'dialog-btn';
    deleteBtn.textContent = '지우기';
    deleteBtn.addEventListener('click', () => finish('delete-cells'));

    const cancelBtn = document.createElement('button');
    cancelBtn.className = 'dialog-btn';
    cancelBtn.textContent = '취소';
    cancelBtn.addEventListener('click', () => finish('cancel'));

    footer.append(keepBtn, deleteBtn, cancelBtn);
    dialog.append(title, body, footer);
    overlay.appendChild(dialog);
    overlay.addEventListener('click', (e) => {
      if (e.target === overlay) finish('cancel');
    });

    function onKeyDown(e: KeyboardEvent): void {
      e.stopPropagation();
      if (e.key === 'Escape') {
        e.preventDefault();
        finish('cancel');
      } else if (e.key === 'Enter') {
        e.preventDefault();
        finish('keep-shape');
      }
    }

    document.addEventListener('keydown', onKeyDown, true);
    document.body.appendChild(overlay);
    keepBtn.focus();
  });
}

export function showCellPasteChoiceDialog(): Promise<CellPasteChoice> {
  return new Promise((resolve) => {
    let resolved = false;
    const finish = (choice: CellPasteChoice) => {
      if (resolved) return;
      resolved = true;
      document.removeEventListener('keydown', onKeyDown, true);
      overlay.remove();
      resolve(choice);
    };

    const overlay = document.createElement('div');
    overlay.className = 'modal-overlay';

    const dialog = document.createElement('div');
    dialog.className = 'dialog-wrap cell-paste-choice-dialog';
    dialog.style.width = '420px';

    const title = document.createElement('div');
    title.className = 'dialog-title';
    title.textContent = '셀 붙이기';

    const closeBtn = document.createElement('button');
    closeBtn.className = 'dialog-close';
    closeBtn.textContent = '\u00D7';
    closeBtn.addEventListener('click', () => finish('cancel'));
    title.appendChild(closeBtn);

    const body = document.createElement('div');
    body.className = 'dialog-body';
    body.style.padding = '18px 20px';
    body.style.lineHeight = '1.55';
    body.innerHTML = `
      <label style="display:flex;align-items:center;gap:10px;margin-bottom:8px;">
        <input type="radio" checked />
        <span>내용만 덮어 쓰기</span>
      </label>
      <div style="color:#666;font-size:13px;">
        선택한 셀의 텍스트를 붙여 넣고 셀 테두리와 배경은 유지합니다.
      </div>
    `;

    const footer = document.createElement('div');
    footer.className = 'dialog-footer';

    const pasteBtn = document.createElement('button');
    pasteBtn.className = 'dialog-btn dialog-btn-primary';
    pasteBtn.textContent = '붙이기';
    pasteBtn.addEventListener('click', () => finish('overwrite'));

    const cancelBtn = document.createElement('button');
    cancelBtn.className = 'dialog-btn';
    cancelBtn.textContent = '취소';
    cancelBtn.addEventListener('click', () => finish('cancel'));

    footer.append(cancelBtn, pasteBtn);
    dialog.append(title, body, footer);
    overlay.appendChild(dialog);
    overlay.addEventListener('click', (e) => {
      if (e.target === overlay) finish('cancel');
    });

    function onKeyDown(e: KeyboardEvent): void {
      e.stopPropagation();
      if (e.key === 'Escape') {
        e.preventDefault();
        finish('cancel');
      } else if (e.key === 'Enter') {
        e.preventDefault();
        finish('overwrite');
      }
    }

    document.addEventListener('keydown', onKeyDown, true);
    document.body.appendChild(overlay);
    pasteBtn.focus();
  });
}
