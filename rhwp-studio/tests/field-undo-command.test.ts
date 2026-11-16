import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync, mkdtempSync, rmSync } from 'node:fs';
import { execFileSync, spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { env } from 'node:process';
import { initSync } from '../../pkg/rhwp.js';
import { WasmBridge } from '../src/core/wasm-bridge';
import type { DocumentPosition } from '../src/core/types';
import {
  DeleteSelectionCommand, DeleteTextCommand, InsertLineBreakCommand, InsertTabCommand,
  InsertTextCommand, MergeNextParagraphCommand, MergeParagraphCommand,
  SplitParagraphCommand, SplitParagraphInCellCommand, MergeParagraphInCellCommand,
  MergeNextParagraphInCellCommand,
  ApplyCharFormatCommand, MoveTableCommand, MovePictureCommand, MoveShapeCommand, SnapshotCommand,
} from '../src/engine/command';
import { EmbedInsertTableCommand, EmbedInsertPictureCommand } from '../src/engine/embed-commands';
import type { EditCommand, OperationDescriptor } from '../src/engine/command';
import { CommandHistory, HistorySuspendedError } from '../src/engine/history';
import { InputHandler } from '../src/engine/input-handler';
import { onPaste, onCut, onCopy } from '../src/engine/input-handler-keyboard';
import { finishMoveDrag } from '../src/engine/input-handler-table';
import { onMouseUp } from '../src/engine/input-handler-mouse';
import { onCompositionStart, onCompositionEnd, onInput, getTextAt, insertTextAtRaw } from '../src/engine/input-handler-text';

initSync({ module: readFileSync(new URL('../../pkg/rhwp_bg.wasm', import.meta.url)) });

test('document reload refuses a retained inverse before changing the reloaded document', () => {
  const wasm = new WasmBridge();
  try {
    wasm.loadDocument(fixture(para('<hp:t>FIRST</hp:t>')), 'first.hwpx');
    const retained = wasm.beginTextEdit(0, 0);
    if (retained === null) throw new Error('history unexpectedly disabled');
    wasm.loadDocument(fixture(para('<hp:t>OTHER</hp:t>')), 'other.hwpx');
    const fresh = wasm.beginTextEdit(0, 0);
    if (fresh === null) throw new Error('history unexpectedly disabled');
    wasm.insertText(0, 0, 0, 'NEW');
    assert.throws(() => wasm.undoEdit(retained));
    assert.equal(wasm.getTextRange(0, 0, 0, 8), 'NEWOTHER');
    roundTrip(wasm);
    wasm.discardEdit(fresh);
  } finally {
    wasm.dispose();
  }
});

test('selection route releases inverses after 1024 history lifecycles', () => {
  const wasm = new WasmBridge();
  const history = new CommandHistory();
  const start = position(0, 0), end = position(0, 1);
  const host = {
    wasm, history,
    cursor: { getSelectionOrdered: () => ({ start, end }), clearSelection() {},
      getPosition: () => start, moveTo() {}, resetPreferredX() {} },
    afterEdit() {}, executeOperation: InputHandler.prototype.executeOperation,
  };
  const deleteSelection = Reflect.get(InputHandler.prototype, 'deleteSelection');
  assert.equal(typeof deleteSelection, 'function');
  try {
    wasm.loadDocument(fixture(para('<hp:t>AB</hp:t>')), 'selection.hwpx');
    const initialCount = wasm.liveEditCount();
    for (let i = 0; i < 1024; i++) {
      deleteSelection.call(host);
      history.undo(wasm);
      history.redo(wasm);
      history.clear(wasm);
      assert.equal(wasm.liveEditCount(), initialCount, `selection lifecycle ${i}`);
      wasm.insertText(0, 0, 0, 'A');
    }
    history.execute(new InsertTextCommand(start, 'X'), wasm);
    assert.equal(wasm.getTextRange(0, 0, 0, 3), 'XAB');
    roundTrip(wasm);
  } finally {
    history.clear(wasm);
    wasm.dispose();
  }
});

const template = new URL('../../samples/hwpx/ref/ref_empty.hwpx', import.meta.url).pathname;
const para = (inner: string): string => `<hp:p id="0" paraPrIDRef="0" styleIDRef="0" pageBreak="0" columnBreak="0" merged="0"><hp:run charPrIDRef="0">${inner}</hp:run></hp:p>`;
const begin = '<hp:ctrl><hp:fieldBegin id="1" type="CLICK_HERE" name="field" editable="1" fieldid="501"/></hp:ctrl>';
const end = '<hp:ctrl><hp:fieldEnd beginIDRef="1" fieldid="501"/></hp:ctrl>';
const body = para(`<hp:t>ZZ</hp:t>${begin}<hp:t>12</hp:t>`) + para(`<hp:t>AB</hp:t>${end}<hp:t>CD</hp:t>`);
const cell = (inner: string): string => para(`<hp:tbl rowCnt="1" colCnt="1" borderFillIDRef="1"><hp:tr><hp:tc borderFillIDRef="1"><hp:subList>${inner}</hp:subList><hp:cellAddr colAddr="0" rowAddr="0"/><hp:cellSpan colSpan="1" rowSpan="1"/><hp:cellSz width="1000" height="1000"/></hp:tc></hp:tr></hp:tbl>`);

function fixture(content: string): Uint8Array {
  return execFileSync('python3', ['-c', `
import io,sys,zipfile
section='<hs:sec xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph" xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section">'+sys.stdin.read()+'</hs:sec>'
out=io.BytesIO()
with zipfile.ZipFile(sys.argv[1]) as src, zipfile.ZipFile(out,'w') as dst:
    for info in src.infolist():
        dst.writestr(info,section.encode() if info.filename=='Contents/section0.xml' else src.read(info.filename))
sys.stdout.buffer.write(out.getvalue())
`, template], { input: content });
}

function fieldSequence(bytes: Uint8Array): string {
  return execFileSync('python3', ['-c', `
import io,json,sys,zipfile,xml.etree.ElementTree as ET
with zipfile.ZipFile(io.BytesIO(sys.stdin.buffer.read())) as src:
    root=ET.fromstring(src.read('Contents/section0.xml'))
def visit(element):
    tag=element.tag.rsplit('}',1)[-1]
    if tag in ('p','fieldBegin','fieldEnd'): out.append([tag,element.attrib if tag!='p' else {}])
    if tag=='t' and element.text: out.append(element.text)
    for child in element: visit(child)
    if tag=='p': out.append('/p')
out=[]
visit(root)
print(json.dumps(out,ensure_ascii=False,sort_keys=True))
`], { input: bytes }).toString().trim();
}

function roundTrip(wasm: WasmBridge): string {
  const saved = wasm.exportHwpx();
  const sequence = fieldSequence(saved);
  const reopened = new WasmBridge();
  try {
    reopened.loadDocument(saved, 'test.hwpx');
    assert.equal(fieldSequence(reopened.exportHwpx()), sequence);
  } finally {
    reopened.dispose();
  }
  return sequence;
}

for (const mutation of ['replaceAll', 'setFieldValue'] as const) {
  test(`revision refuses redo after ${mutation} through WasmBridge`, () => {
    const wasm = new WasmBridge(), history = new CommandHistory();
    try {
      wasm.loadDocument(fixture(para(`${begin}<hp:t>ABC</hp:t>${end}`)));
      history.execute(new InsertTextCommand(position(0, 1), 'OLD'), wasm);
      assert.ok(history.undo(wasm));
      if (mutation === 'replaceAll') wasm.replaceAll('ABC', 'NEW', true);
      else wasm.setFieldValue(501, 'NEW');
      assert.equal(history.redo(wasm), null);
      assert.equal(wasm.getTextRange(0, 0, 0, 3), 'NEW');
      roundTrip(wasm);
    } finally { history.clear(wasm); wasm.dispose(); }
  });
}

test('selection paste has one undo entry and releases its inner inverses', () => {
  const wasm = new WasmBridge(), history = new CommandHistory();
  let current = position(0, 0), selected = true;
  const host = {
    wasm, history, active: true,
    cursor: { isInPictureObjectSelection: () => false, isInTableObjectSelection: () => false,
      hasSelection: () => selected, getPosition: () => current,
      getSelectionOrdered: () => selected ? { start: position(0, 0), end: position(0, 1) } : null,
      clearSelection() { selected = false; }, moveTo(p: DocumentPosition) { current = p; }, resetPreferredX() {} },
    afterEdit() {}, executeOperation: InputHandler.prototype.executeOperation,
    deleteSelection: Reflect.get(InputHandler.prototype, 'deleteSelection'),
  };
  try {
    wasm.loadDocument(fixture(para(`${begin}<hp:t>AB</hp:t>${end}`) + para('<hp:t>X</hp:t>')));
    wasm.copySelection(0, 1, 0, 1, 1);
    onPaste.call(host, Object.assign(new Event('paste'), { clipboardData: null }));
    assert.equal(wasm.getTextRange(0, 0, 0, 2), 'XB');
    const edited = roundTrip(wasm);
    assert.ok(history.undo(wasm));
    assert.equal(wasm.getTextRange(0, 0, 0, 2), 'AB');
    assert.equal(history.canUndo(), false);
    assert.ok(history.redo(wasm));
    assert.equal(roundTrip(wasm), edited);
    history.clear(wasm);
    assert.equal(wasm.liveEditCount(), 0);
  } finally { history.clear(wasm); wasm.dispose(); }
});

const position = (paragraphIndex: number, charOffset: number): DocumentPosition => ({ sectionIndex: 0, paragraphIndex, charOffset });

test('model mutation API is private and live document writes require the gate', { timeout: 60_000 }, () => {
  execFileSync('cargo', ['rustdoc', '--offline', '--lib', '--', '-Z', 'unstable-options', '--output-format', 'json'],
    { cwd: new URL('../../', import.meta.url), env: { ...env, RUSTC_BOOTSTRAP: '1' }, stdio: 'pipe' });
  type Item = { readonly name: string; readonly inner: {
    readonly struct?: { readonly impls: readonly number[] };
    readonly impl?: { readonly trait: unknown; readonly items: readonly number[] };
    readonly function?: { readonly sig: { readonly inputs: unknown; readonly output: unknown } };
  } };
  const api: { readonly index: Readonly<Record<string, Item>> } = JSON.parse(
    readFileSync(new URL('../../target/doc/rhwp.json', import.meta.url), 'utf8'));
  const hasMutableBorrow = (value: unknown): boolean => {
    if (typeof value !== 'object' || value === null) return false;
    if ('borrowed_ref' in value && typeof value.borrowed_ref === 'object' && value.borrowed_ref !== null
      && 'is_mutable' in value.borrowed_ref && value.borrowed_ref.is_mutable === true) return true;
    return Object.values(value).some(hasMutableBorrow);
  };
  const owners = Object.values(api.index).filter(item => item.inner.struct
    && ['Paragraph', 'Table', 'Section'].includes(item.name));
  assert.equal(owners.length, 3);
  for (const owner of owners) {
    for (const id of owner.inner.struct?.impls ?? []) {
      const implementation = api.index[String(id)].inner.impl;
      if (!implementation || implementation.trait !== null) continue;
      for (const methodId of implementation.items) {
        const method = api.index[String(methodId)];
        const signature = method.inner.function?.sig;
        assert.equal(hasMutableBorrow(signature), false, `${owner.name}::${method.name} exposes mutable model access`);
      }
    }
  }
  const directory = mkdtempSync(join(tmpdir(), 'rhwp-mutation-api-'));
  const root = new URL('../../', import.meta.url).pathname;
  try {
    const args = ['--edition=2021', '--crate-type=lib', '--crate-name=mutation_boundary', '-',
      '-L', `dependency=${root}target/debug/deps`, '--extern', `rhwp=${root}target/debug/deps/librhwp.rlib`,
      '-o', join(directory, 'boundary.rlib')];
    const read = spawnSync('rustc', args, { input: 'pub fn read(core: &rhwp::DocumentCore) { let _ = core.document().sections.len(); }', encoding: 'utf8' });
    assert.equal(read.status, 0, read.stderr);
    const write = spawnSync('rustc', args, { input: 'pub fn write(core: &mut rhwp::DocumentCore) { core.document.sections.clear(); }', encoding: 'utf8' });
    assert.equal(write.status, 1);
    assert.match(write.stderr, /E0616/);
    console.log('MUTATION_API_BOUNDARY owners=3 public_mutators=0 raw_write_refused=true');
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

for (const record of [false, true]) {
  test(`snapshot rejects inner ${record ? 'record' : 'execute'} attempts and releases captures`, () => {
    const wasm = new WasmBridge(), history = new CommandHistory();
    try {
      wasm.loadDocument(fixture(para('<hp:t>AB</hp:t>')));
      const command = new SnapshotCommand('nestedRecord', position(0, 0), position(0, 0), live => {
        assert.equal(live.beginTextEdit(0, 0), null);
        const inner = new InsertTextCommand(position(0, 0), 'X');
        if (record) history.recordWithoutExecute(inner, live);
        else history.execute(inner, live);
        return position(0, 1);
      });
      assert.throws(() => history.execute(command, wasm), HistorySuspendedError);
      assert.equal(history.canUndo(), false);
      assert.equal(history.canRedo(), false);
      assert.equal(wasm.liveEditCount(), 0);
      assert.equal(wasm.getTextRange(0, 0, 0, 2), 'AB');
      history.execute(new InsertTextCommand(position(0, 0), 'Y'), wasm);
      assert.ok(history.undo(wasm));
      roundTrip(wasm);
    } finally { history.clear(wasm); wasm.dispose(); }
  });
}

test('inverse revisions chain across snapshot undo and redo', () => {
  const wasm = new WasmBridge(), history = new CommandHistory();
  try {
    wasm.loadDocument(fixture(para('<hp:t>AB</hp:t>')));
    history.execute(new InsertTextCommand(position(0, 0), 'X'), wasm);
    history.execute(new SnapshotCommand('replace', position(0, 0), position(0, 0), live => {
      live.replaceAll('AB', 'CD', true);
      return position(0, 0);
    }), wasm);
    history.execute(new InsertTextCommand(position(0, 0), 'Y'), wasm);
    assert.equal(wasm.getTextRange(0, 0, 0, 4), 'YXCD');
    for (const text of ['XCD', 'XAB', 'AB']) {
      assert.ok(history.undo(wasm));
      assert.equal(wasm.getTextRange(0, 0, 0, 8), text);
      roundTrip(wasm);
    }
    assert.equal(history.canUndo(), false);
    for (const text of ['XAB', 'XCD', 'YXCD']) {
      assert.ok(history.redo(wasm));
      assert.equal(wasm.getTextRange(0, 0, 0, 8), text);
      roundTrip(wasm);
    }
    assert.equal(history.canRedo(), false);
  } finally { history.clear(wasm); wasm.dispose(); }
});

test('typing after an uncaptured mutation starts a new inverse group', () => {
  const wasm = new WasmBridge(), history = new CommandHistory();
  try {
    wasm.loadDocument(fixture(para(`${begin}<hp:t>ABC</hp:t>${end}`)));
    history.execute(new InsertTextCommand(position(0, 1), 'X'), wasm);
    wasm.setFieldValue(501, 'NEW');
    history.execute(new InsertTextCommand(position(0, 2), 'Y'), wasm);
    assert.equal(wasm.getTextRange(0, 0, 0, 4), 'NEYW');
    assert.ok(history.undo(wasm));
    assert.equal(wasm.getTextRange(0, 0, 0, 3), 'NEW');
    assert.equal(history.undo(wasm), null);
    assert.equal(wasm.getTextRange(0, 0, 0, 3), 'NEW');
    roundTrip(wasm);
  } finally { history.clear(wasm); wasm.dispose(); }
});

test('IME in a selection has one snapshot entry including cancellation', () => {
  for (const cancel of [false, true]) {
    const wasm = new WasmBridge(), history = new CommandHistory();
    let current = position(0, 0), selected = true;
    const host = {
      wasm, history, active: true, isComposing: false, compositionAnchor: null,
      compositionLength: 0, compositionEditToken: null, _isIOS: false,
      textarea: { value: '' }, caret: { hideComposition() {} },
      cursor: { isInHeaderFooter: () => false, isInFootnote: () => false,
        hasSelection: () => selected, getPosition: () => current,
        getSelectionOrdered: () => selected ? { start: position(0, 0), end: position(0, 1) } : null,
        clearSelection() { selected = false; }, moveTo(p: DocumentPosition) { current = p; }, resetPreferredX() {} },
      afterEdit() {}, executeOperation: InputHandler.prototype.executeOperation,
      deleteSelection: Reflect.get(InputHandler.prototype, 'deleteSelection'),
      getTextAt, insertTextAtRaw, deleteTextAt: (p: DocumentPosition, n: number) => wasm.deleteText(0, 0, p.charOffset, n),
    };
    try {
      wasm.loadDocument(fixture(para(`${begin}<hp:t>AB</hp:t>${end}`)));
      const original = roundTrip(wasm);
      onCompositionStart.call(host);
      host.textarea.value = '가'; onInput.call(host);
      assert.equal(wasm.liveEditCount(), 0);
      host.textarea.value = cancel ? '' : '각'; onInput.call(host);
      onCompositionEnd.call(host);
      const edited = roundTrip(wasm);
      if (cancel) {
        assert.equal(edited, original);
        assert.equal(history.canUndo(), false);
      } else {
        assert.equal(wasm.getTextRange(0, 0, 0, 2), '각B');
        assert.ok(history.undo(wasm));
        assert.equal(roundTrip(wasm), original);
        assert.equal(history.canUndo(), false);
        assert.ok(history.redo(wasm));
        assert.equal(roundTrip(wasm), edited);
      }
      history.clear(wasm);
      assert.equal(wasm.liveEditCount(), 0);
    } finally { history.clear(wasm); wasm.dispose(); }
  }
});

for (const selection of ['tail', 'whole', 'cell'] as const) {
  test(`control membership follows ${selection} selection through WasmBridge`, () => {
    const wasm = new WasmBridge();
    try {
      const table = cell(para(`${begin}<hp:t>SECRET</hp:t>${end}`))
        .replace('</hp:tbl>', '</hp:tbl><hp:t>ZZ</hp:t>');
      wasm.loadDocument(fixture(table + para('<hp:t>TARGET</hp:t>')));
      if (selection === 'cell') wasm.copySelectionInCell(0, 0, 0, 0, 0, 0, 0, 3);
      else wasm.copySelection(0, 0, selection === 'tail' ? 1 : 0, 0, 2);
      wasm.pasteInternal(0, 1, 0);
      assert.equal(wasm.getTextRange(0, 1, 0, 20), `${selection === 'tail' ? 'Z' : selection === 'whole' ? 'ZZ' : 'SEC'}TARGET`);
      const fields = wasm.getFieldList();
      assert.equal(fields.length, selection === 'whole' ? 2 : 1);
      assert.equal(new Set(fields.map(field => field.fieldId)).size, fields.length);
      const sequence = roundTrip(wasm);
      assert.equal((sequence.match(/fieldBegin/g) ?? []).length, fields.length);
    } finally { wasm.dispose(); }
  });
}

test('cell mutation invalidates redo through WasmBridge', () => {
  const wasm = new WasmBridge(), history = new CommandHistory();
  try {
    wasm.loadDocument(fixture(cell(para(`${begin}<hp:t>ABC</hp:t>${end}`))));
    const target: DocumentPosition = { ...position(0, 1), parentParaIndex: 0, controlIndex: 0, cellIndex: 0, cellParaIndex: 0 };
    history.execute(new InsertTextCommand(target, 'OLD'), wasm);
    assert.ok(history.undo(wasm));
    wasm.insertTextInCell(0, 0, 0, 0, 0, 0, 'NEW');
    assert.equal(history.redo(wasm), null);
    assert.equal(wasm.getTextInCell(0, 0, 0, 0, 0, 0, 6), 'NEWABC');
    roundTrip(wasm);
  } finally { history.clear(wasm); wasm.dispose(); }
});

test('object cut snapshot has one undo entry', () => {
  const wasm = new WasmBridge(), history = new CommandHistory();
  const host = {
    wasm, history, active: true,
    cursor: { isInPictureObjectSelection: () => true,
      getSelectedPictureRef: () => ({ type: 'shape', sec: 0, ppi: 0, ci: 0 }),
      moveOutOfSelectedPicture() {}, getPosition: () => position(0, 0), moveTo() {}, resetPreferredX() {} },
    eventBus: { emit() {} }, afterEdit() {}, executeOperation: InputHandler.prototype.executeOperation,
    onCopy(event: ClipboardEvent) { onCopy.call(this, event); },
  };
  try {
    wasm.loadDocument(fixture(para(`<hp:rect><hp:drawText><hp:subList>${para(`${begin}<hp:t>SECRET</hp:t>${end}`)}</hp:subList></hp:drawText></hp:rect>`)));
    const original = roundTrip(wasm);
    onCut.call(host, Object.assign(new Event('cut'), { clipboardData: null }));
    assert.equal(wasm.getFieldList().length, 0);
    roundTrip(wasm);
    assert.ok(history.undo(wasm));
    assert.equal(roundTrip(wasm), original);
    assert.equal(history.canUndo(), false);
    assert.ok(history.redo(wasm));
    assert.equal(wasm.getFieldList().length, 0);
    history.clear(wasm);
    assert.equal(wasm.liveEditCount(), 0);
  } finally { history.clear(wasm); wasm.dispose(); }
});

test('table drag drop records one undo entry', () => {
  const wasm = new WasmBridge(), history = new CommandHistory();
  try {
    wasm.loadDocument(fixture(cell(para(`${begin}<hp:t>SECRET</hp:t>${end}`))));
    const before = wasm.getTableProperties(0, 0, 0);
    const originalH = before.horzOffset ?? 0;
    const moved = wasm.moveTableOffset(0, 0, 0, 10, 20);
    const host = {
      wasm, history, isMoveDragging: true, dragRafId: 0,
      moveDragState: { totalDeltaH: 10, totalDeltaV: 20, startPpi: 0,
        tableRef: { sec: 0, ppi: moved.ppi, ci: moved.ci } },
      container: { style: { cursor: '' } },
      executeOperation: InputHandler.prototype.executeOperation,
      finishMoveDrag() { finishMoveDrag.call(this); },
    };
    Reflect.apply(onMouseUp, host, [new Event('mouseup')]);
    assert.ok(history.undo(wasm));
    const restored = wasm.getTableProperties(0, 0, 0);
    assert.equal(restored.horzOffset, before.horzOffset);
    assert.equal(restored.vertOffset, before.vertOffset);
    assert.equal(history.canUndo(), false);
    roundTrip(wasm);
    assert.ok(history.redo(wasm));
    assert.equal(wasm.getTableProperties(0, moved.ppi, moved.ci).horzOffset, originalH + 10);
    roundTrip(wasm);
    history.clear(wasm);
    assert.equal(wasm.liveEditCount(), 0);
  } finally { history.clear(wasm); wasm.dispose(); }
});
const cases: readonly { readonly name: string; readonly make: () => EditCommand }[] = [
  { name: 'insert at begin', make: () => new InsertTextCommand(position(0, 2), 'XY') },
  { name: 'insert at end', make: () => new InsertTextCommand(position(1, 2), 'XY') },
  { name: 'delete before begin', make: () => new DeleteTextCommand(position(0, 0), 2, 'forward') },
  { name: 'delete after end', make: () => new DeleteTextCommand(position(1, 2), 2, 'backward') },
  { name: 'line break', make: () => new InsertLineBreakCommand(position(1, 2)) },
  { name: 'tab', make: () => new InsertTabCommand(position(1, 2)) },
  { name: 'split', make: () => new SplitParagraphCommand(position(0, 2)) },
  { name: 'merge backward', make: () => new MergeParagraphCommand(position(1, 0)) },
  { name: 'merge forward', make: () => new MergeNextParagraphCommand(position(0, 4)) },
  { name: 'selection', make: () => new DeleteSelectionCommand(position(0, 0), position(0, 2)) },
];

for (const entry of cases) {
  test(`field sequence survives ${entry.name}, undo and redo through WASM`, () => {
    const wasm = new WasmBridge();
    const history = new CommandHistory();
    try {
      wasm.loadDocument(fixture(body), 'test.hwpx');
      const original = roundTrip(wasm);
      history.execute(entry.make(), wasm);
      const edited = roundTrip(wasm);
      history.undo(wasm);
      assert.equal(roundTrip(wasm), original);
      history.redo(wasm);
      assert.equal(roundTrip(wasm), edited);
    } finally {
      history.clear(wasm);
      wasm.dispose();
    }
  });
}


test('Studio commands release their inverses after execute undo redo and clear', { timeout: 20_000 }, () => {
  const routes = new Map<string, { readonly content: string; readonly make: (wasm: WasmBridge) => EditCommand }>();
  for (const entry of cases) routes.set(entry.make().constructor.name, { content: body, make: entry.make });
  const cellPosition: DocumentPosition = {
    ...position(0, 2), parentParaIndex: 0, controlIndex: 0, cellIndex: 0, cellParaIndex: 1,
  };
  const png = Uint8Array.from(atob('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jqK8AAAAASUVORK5CYII='), ch => ch.charCodeAt(0));
  routes.set('SplitParagraphInCellCommand', { content: cell(body), make: () => new SplitParagraphInCellCommand(cellPosition) });
  routes.set('MergeParagraphInCellCommand', { content: cell(body), make: () => new MergeParagraphInCellCommand({ ...cellPosition, charOffset: 0 }) });
  routes.set('MergeNextParagraphInCellCommand', { content: cell(body), make: () => new MergeNextParagraphInCellCommand({ ...cellPosition, cellParaIndex: 0 }) });
  routes.set('ApplyCharFormatCommand', { content: body, make: () => new ApplyCharFormatCommand(position(1, 0), position(1, 2), { bold: true }) });
  routes.set('MoveTableCommand', { content: cell(body), make: () => new MoveTableCommand(0, 0, 0, 10, 10, 0, 0) });
  routes.set('MovePictureCommand', { content: body, make: wasm => {
    const result = wasm.insertPicture(0, 0, 0, png, 100, 100, 1, 1, 'png');
    const props = wasm.getPictureProperties(0, result.paraIdx, result.controlIdx);
    return new MovePictureCommand(0, result.paraIdx, result.controlIdx, 10, 10, props.horzOffset, props.vertOffset);
  }});
  routes.set('MoveShapeCommand', { content: para(`<hp:rect><hp:drawText><hp:subList>${body}</hp:subList></hp:drawText></hp:rect>`), make: wasm => {
    const props = wasm.getShapeProperties(0, 0, 0);
    return new MoveShapeCommand(0, 0, 0, 10, 10, props.horzOffset, props.vertOffset);
  }});
  routes.set('SnapshotCommand', { content: body, make: wasm => {
    wasm.copySelection(0, 1, 0, 1, 1);
    return new SnapshotCommand('pasteInternal', position(1, 4), position(1, 5), bridge => {
      bridge.pasteInternal(0, 1, 4);
      return position(1, 5);
    });
  }});
  routes.set('EmbedInsertTableCommand', { content: body, make: () => new EmbedInsertTableCommand(position(1, 4), { cells: [[{ text: 'CELL' }]] }) });
  routes.set('EmbedInsertPictureCommand', { content: body, make: () => new EmbedInsertPictureCommand(position(1, 4), { imageRef: { kind: 'binData', bytes: png, mime: 'image/png' } }) });
  const discovered = [...routes.keys()].sort();
  for (const name of discovered) {
    const route = routes.get(name);
    if (!route) throw new Error(`missing lifecycle for ${name}`);
    const wasm = new WasmBridge(), history = new CommandHistory();
    try {
      wasm.loadDocument(fixture(route.content), 'lifecycle.hwpx');
      const command = route.make(wasm);
      const initial = wasm.liveEditCount();
      history.execute(command, wasm);
      roundTrip(wasm);
      assert.ok(history.undo(wasm), `${name}: undo`);
      roundTrip(wasm);
      assert.ok(history.redo(wasm), `${name}: redo`);
      roundTrip(wasm);
      history.clear(wasm);
      assert.equal(wasm.liveEditCount(), initial, name);
    } finally {
      history.clear(wasm);
      wasm.dispose();
    }
  }
  console.log(`STUDIO_COMMAND_LIFECYCLES=${discovered.length} live-token delta=0`);
});

test('IME cancellation releases its provisional inverse', () => {
  const wasm = new WasmBridge(), history = new CommandHistory();
  let cursor = position(1, 2);
  const host = {
    wasm, active: true, isComposing: false, compositionAnchor: null,
    compositionLength: 0, compositionEditToken: null, _isIOS: false,
    textarea: { value: '' }, caret: { hideComposition() {} },
    cursor: { hasSelection: () => false, isInHeaderFooter: () => false, isInFootnote: () => false,
      getPosition: () => cursor, moveTo: (pos: DocumentPosition) => { cursor = pos; } },
    afterEdit() {}, getTextAt, insertTextAtRaw,
    executeOperation(desc: OperationDescriptor) {
      if (desc.kind === 'record') history.recordWithoutExecute(desc.command, wasm);
    },
  };
  try {
    wasm.loadDocument(fixture(body), 'cancel.hwpx');
    const initial = wasm.liveEditCount(), original = roundTrip(wasm);
    onCompositionStart.call(host);
    for (const text of ['ㄱ', '가', '']) {
      host.textarea.value = text;
      onInput.call(host);
    }
    onCompositionEnd.call(host);
    assert.equal(roundTrip(wasm), original);
    history.clear(wasm);
    assert.equal(wasm.liveEditCount(), initial);
    assert.equal(history.canUndo(), false);
  } finally { history.clear(wasm); wasm.dispose(); }
});

test('iOS provisional cancellation releases its inverse when input deactivates', () => {
  const wasm = new WasmBridge(), history = new CommandHistory();
  let cursor = position(1, 2);
  const host = {
    wasm, history, active: true, isComposing: false, _isIOS: true,
    _iosAnchor: null, _iosLength: 0, _iosEditToken: null, _iosInputTimer: undefined,
    compositionEditToken: null, compositionSelectionSnapshot: null,
    textarea: { value: '', focus() {} }, caret: { hide() {} },
    fieldMarker: { hide() {} }, selectionRenderer: { clear() {} },
    cursor: { isInHeaderFooter: () => false, isInFootnote: () => false, clearSelection() {},
      getPosition: () => cursor, moveTo: (pos: DocumentPosition) => { cursor = pos; } },
    afterEdit() {}, insertTextAtRaw,
  };
  try {
    wasm.loadDocument(fixture(body), 'ios-cancel.hwpx');
    const initial = wasm.liveEditCount(), original = roundTrip(wasm);
    for (const text of ['ㄱ', '가', '']) {
      host.textarea.value = text;
      onInput.call(host);
    }
    clearTimeout(host._iosInputTimer);
    InputHandler.prototype.deactivate.call(host);
    assert.equal(roundTrip(wasm), original);
    assert.equal(wasm.liveEditCount(), initial);
  } finally { clearTimeout(host._iosInputTimer); history.clear(wasm); wasm.dispose(); }
});

test('history treats a refused inverse as a no-op and retains its entry', () => {
  const wasm = new WasmBridge(), history = new CommandHistory();
  try {
    wasm.loadDocument(fixture(body), 'original.hwpx');
    history.execute(new InsertTextCommand(position(1, 4), 'OLD'), wasm);
    wasm.loadDocument(fixture(body), 'reload.hwpx');
    const current = roundTrip(wasm);
    assert.equal(history.undo(wasm), null);
    assert.equal(history.canUndo(), true);
    assert.equal(roundTrip(wasm), current);
  } finally { history.clear(wasm); wasm.dispose(); }
});

test('missing entropy disables handles while the document still opens and edits', () => {
  const descriptor = Object.getOwnPropertyDescriptor(globalThis, 'crypto');
  const wasm = new WasmBridge(), history = new CommandHistory();
  try {
    wasm.loadDocument(fixture(body), 'with-entropy.hwpx');
    history.execute(new InsertTextCommand(position(1, 4), 'OLD'), wasm);
    Object.defineProperty(globalThis, 'crypto', { configurable: true, value: {
      getRandomValues() { throw new Error('entropy unavailable'); },
    }});
    wasm.loadDocument(fixture(body), 'without-entropy.hwpx');
    assert.equal(history.undo(wasm), null);
    history.clear(wasm);
    assert.equal(wasm.beginTextEdit(0, 1), null);
    assert.match(wasm.editHistoryDisabledReason ?? '', /EDIT_HISTORY_UNAVAILABLE:/);
    history.execute(new InsertTextCommand(position(1, 4), 'NEW'), wasm);
    assert.equal(wasm.getTextRange(0, 1, 0, 7), 'ABCDNEW');
    assert.equal(history.canUndo(), false);
    assert.equal(wasm.liveEditCount(), 0);
    roundTrip(wasm);
  } finally {
    if (descriptor) Object.defineProperty(globalThis, 'crypto', descriptor);
    history.clear(wasm);
    wasm.dispose();
  }
});

for (const depth of [0, 1, 2]) {
  test(`clipboard field fragments keep complete owners at cell depth ${depth}`, () => {
    const wasm = new WasmBridge();
    let content = body + para('<hp:t>TARGET</hp:t>');
    for (let i = 0; i < depth; i++) content = cell(content);
    try {
      wasm.loadDocument(fixture(content), 'clipboard.hwpx');
      if (depth === 0) {
        wasm.copySelection(0, 0, 2, 1, 2);
        wasm.pasteInternal(0, 2, 0);
      } else if (depth === 1) {
        wasm.copySelectionInCell(0, 0, 0, 0, 0, 2, 1, 2);
        wasm.pasteInternalInCell(0, 0, 0, 0, 2, 0);
      } else {
        const path = [{ controlIndex: 0, cellIndex: 0, cellParaIndex: 0 }, { controlIndex: 0, cellIndex: 0, cellParaIndex: 0 }];
        wasm.copySelectionInCellByPath(0, 0, JSON.stringify(path), 2, 1, 2);
        path[1].cellParaIndex = 2;
        wasm.pasteInternalInCellByPath(0, 0, JSON.stringify(path), 0);
      }
      assert.equal((roundTrip(wasm).match(/"fieldBegin"/g) ?? []).length, 2);
    } finally { wasm.dispose(); }
  });
}

for (const depth of [1, 2]) {
  for (const operation of ['insert', 'delete', 'split', 'merge', 'merge next']) {
    test(`cell depth ${depth}: ${operation} has an exact field inverse`, () => {
      const wasm = new WasmBridge();
      const history = new CommandHistory();
      const pos: DocumentPosition = {
        ...position(0, 2), parentParaIndex: 0, controlIndex: 0, cellIndex: 0, cellParaIndex: 1,
        cellPath: Array.from({ length: depth }, (_, i) => ({ controlIndex: 0, cellIndex: 0, cellParaIndex: i === depth - 1 ? 1 : 0 })),
      };
      const command = (): EditCommand => {
        switch (operation) {
          case 'insert': return new InsertTextCommand(pos, 'XY');
          case 'delete': return new DeleteTextCommand(pos, 2, 'forward');
          case 'split': return new SplitParagraphInCellCommand(pos);
          case 'merge': return new MergeParagraphInCellCommand({ ...pos, charOffset: 0 });
          case 'merge next': return new MergeNextParagraphInCellCommand({ ...pos, cellParaIndex: 0, charOffset: 4, cellPath: pos.cellPath?.map(p => ({ ...p, cellParaIndex: 0 })) });
          default: throw new Error(`unknown operation: ${operation}`);
        }
      };
      try {
        wasm.loadDocument(fixture(depth === 1 ? cell(body) : cell(cell(body))), 'test.hwpx');
        const original = roundTrip(wasm);
        history.execute(command(), wasm);
        const edited = roundTrip(wasm);
        history.undo(wasm);
        assert.equal(roundTrip(wasm), original);
        history.redo(wasm);
        assert.equal(roundTrip(wasm), edited);
      } finally {
        history.clear(wasm);
        wasm.dispose();
      }
    });
  }
}

test('merged deletion and insertion commands retain one exact inverse and redo state', () => {
  for (const deletion of [false, true]) {
    const wasm = new WasmBridge();
    const history = new CommandHistory();
    try {
      wasm.loadDocument(fixture(body), 'test.hwpx');
      const original = roundTrip(wasm);
      const pos = position(1, 2);
      if (deletion) {
        history.execute(new DeleteTextCommand(pos, 1, 'forward', undefined, 1), wasm);
        history.execute(new DeleteTextCommand(pos, 1, 'forward', undefined, 2), wasm);
      } else {
        history.execute(new InsertTextCommand(pos, 'X', 1), wasm);
        history.execute(new InsertTextCommand(position(1, 3), 'Y', 2), wasm);
      }
      const edited = roundTrip(wasm);
      history.undo(wasm);
      assert.equal(roundTrip(wasm), original);
      assert.equal(history.canUndo(), false);
      history.redo(wasm);
      assert.equal(roundTrip(wasm), edited);
    } finally {
      history.clear(wasm);
      wasm.dispose();
    }
  }
});

test('IME provisional replacement and recorded command restore the original field sequence', () => {
  const wasm = new WasmBridge();
  const history = new CommandHistory();
  let cursor = position(1, 2);
  const host = {
    wasm, active: true, isComposing: false, compositionAnchor: null,
    compositionLength: 0, compositionEditToken: null, _isIOS: false,
    textarea: { value: '' }, caret: { hideComposition() {} },
    cursor: {
      hasSelection: () => false, isInHeaderFooter: () => false, isInFootnote: () => false,
      getPosition: () => cursor, moveTo: (pos: DocumentPosition) => { cursor = pos; },
    },
    afterEdit() {}, getTextAt, insertTextAtRaw,
    executeOperation(desc: OperationDescriptor) {
      assert.equal(desc.kind, 'record');
      if (desc.kind === 'record') history.recordWithoutExecute(desc.command, wasm);
    },
  };
  try {
    wasm.loadDocument(fixture(body), 'test.hwpx');
    const original = roundTrip(wasm);
    onCompositionStart.call(host);
    for (const text of ['X', 'XY', 'XYZ']) {
      host.textarea.value = text;
      onInput.call(host);
      roundTrip(wasm);
    }
    onCompositionEnd.call(host);
    const edited = roundTrip(wasm);
    history.undo(wasm);
    assert.equal(roundTrip(wasm), original);
    history.redo(wasm);
    assert.equal(roundTrip(wasm), edited);
  } finally {
    history.clear(wasm);
    wasm.dispose();
  }
});
