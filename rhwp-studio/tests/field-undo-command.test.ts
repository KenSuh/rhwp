import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync, readdirSync, mkdtempSync, rmSync } from 'node:fs';
import { execFileSync, spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { env } from 'node:process';
import { inflateRawSync } from 'node:zlib';
import ts from 'typescript';
import { initSync } from '../../pkg/rhwp.js';
import { WasmBridge, InvalidEditHandleError } from '../src/core/wasm-bridge';
import type { CharProperties, DocumentPosition } from '../src/core/types';
import {
  DeleteSelectionCommand, DeleteTextCommand, InsertLineBreakCommand, InsertTabCommand,
  InsertTextCommand, MergeNextParagraphCommand, MergeParagraphCommand,
  SplitParagraphCommand, SplitParagraphInCellCommand, MergeParagraphInCellCommand,
  MergeNextParagraphInCellCommand,
  ApplyCharFormatCommand, MoveTableCommand, MovePictureCommand, MoveShapeCommand, SnapshotCommand,
  ObjectGestureRecorder, documentEpochOf,
} from '../src/engine/command';
import { EmbedInsertTableCommand, EmbedInsertPictureCommand } from '../src/engine/embed-commands';
import type { EditCommand, OperationDescriptor } from '../src/engine/command';
import { CommandHistory, HistorySuspendedError, MAX_HISTORY_SIZE, stateLabelOf } from '../src/engine/history';
import { InputHandler } from '../src/engine/input-handler';
import { onPaste, onCut, onCopy, onKeyDown } from '../src/engine/input-handler-keyboard';
import {
  cleanupResizeDrag,
  finishMoveDrag,
  finishResizeDrag,
  hasOpenPointerGesture,
  moveSelectedPicture,
  moveSelectedTable,
  resizeCellByKeyboard,
  resizeTableProportional,
  startResizeDrag,
  updateMoveDrag,
  updateResizeDrag,
} from '../src/engine/input-handler-table';
import {
  cleanupPictureResizeDrag,
  findPictureBbox,
  finishPictureMoveDrag,
  finishPictureResizeDrag,
  finishPictureRotateDrag,
  getObjectProperties,
  setObjectProperties,
  updatePictureMoveDrag,
  updatePictureResizeDrag,
  updatePictureRotateDrag,
} from '../src/engine/input-handler-picture';
import {
  finishLineEndpointDrag,
  onClick,
  onMouseMove,
  onMouseUp,
  updateLineEndpointDrag,
} from '../src/engine/input-handler-mouse';
import { TableResizeRenderer } from '../src/engine/table-resize-renderer';
import { onCompositionStart, onCompositionEnd, onInput, getTextAt, insertTextAtRaw } from '../src/engine/input-handler-text';
import { HwpCtrl } from '../src/hwpctl/index';

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

test('history treats a refused inverse as a no-op and truncates unreachable entries', () => {
  const wasm = new WasmBridge(), history = new CommandHistory();
  try {
    wasm.loadDocument(fixture(body), 'original.hwpx');
    history.execute(new InsertTextCommand(position(1, 4), 'OLD'), wasm);
    wasm.loadDocument(fixture(body), 'reload.hwpx');
    const current = roundTrip(wasm);
    assert.equal(history.undo(wasm), null);
    assert.equal(history.canUndo(), false);
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

function readSectionXmlFast(hwpx: Uint8Array): string {
  const view = new DataView(hwpx.buffer, hwpx.byteOffset, hwpx.byteLength);
  const decoder = new TextDecoder();
  let eocd = -1;
  for (let p = hwpx.byteLength - 22; p >= Math.max(0, hwpx.byteLength - 65557); p--) {
    if (view.getUint32(p, true) === 0x06054b50) {
      eocd = p;
      break;
    }
  }
  assert.notEqual(eocd, -1, 'missing ZIP EOCD');
  const cdCount = view.getUint16(eocd + 10, true);
  let offset = view.getUint32(eocd + 16, true);
  for (let i = 0; i < cdCount; i++) {
    assert.equal(view.getUint32(offset, true), 0x02014b50);
    const method = view.getUint16(offset + 10, true);
    const compSize = view.getUint32(offset + 20, true);
    const nameLen = view.getUint16(offset + 28, true);
    const extraLen = view.getUint16(offset + 30, true);
    const commentLen = view.getUint16(offset + 32, true);
    const localOffset = view.getUint32(offset + 42, true);
    const name = decoder.decode(hwpx.subarray(offset + 46, offset + 46 + nameLen));
    if (name === 'Contents/section0.xml') {
      const localNameLen = view.getUint16(localOffset + 26, true);
      const localExtraLen = view.getUint16(localOffset + 28, true);
      const dataStart = localOffset + 30 + localNameLen + localExtraLen;
      const slice = hwpx.subarray(dataStart, dataStart + compSize);
      if (method === 0) return decoder.decode(slice);
      if (method === 8) return decoder.decode(inflateRawSync(slice));
      throw new Error(`unsupported zip compression method ${method}`);
    }
    offset += 46 + nameLen + extraLen + commentLen;
  }
  throw new Error('Contents/section0.xml not found');
}

function countXmlTag(xml: string, localTag: string): number {
  const re = new RegExp(`<hp:${localTag}\\b`, 'g');
  return (xml.match(re) ?? []).length;
}

function makeStudioHost(
  wasm: WasmBridge,
  history: CommandHistory,
  initialSelection: { start: DocumentPosition; end: DocumentPosition } | null,
) {
  let current: DocumentPosition = initialSelection ? { ...initialSelection.start } : position(0, 0);
  let selection = initialSelection ? { start: { ...initialSelection.start }, end: { ...initialSelection.end } } : null;
  const host: any = {
    wasm,
    history,
    active: true,
    isComposing: false,
    compositionAnchor: null,
    compositionLength: 0,
    compositionEditToken: null,
    compositionSelectionSnapshot: null,
    _isIOS: false,
    textarea: { value: '' },
    caret: { hideComposition() {} },
    eventBus: { emit() {} },
    cursor: {
      isInPictureObjectSelection: () => false,
      isInTableObjectSelection: () => false,
      isInHeaderFooter: () => false,
      isInFootnote: () => false,
      hasSelection: () => selection !== null,
      getPosition: () => ({ ...current }),
      getSelectionOrdered: () => (selection ? { start: { ...selection.start }, end: { ...selection.end } } : null),
      clearSelection() { selection = null; },
      moveTo(next: DocumentPosition) { current = { ...next }; },
      resetPreferredX() {},
      setSelection(next: { start: DocumentPosition; end: DocumentPosition } | null) {
        selection = next ? { start: { ...next.start }, end: { ...next.end } } : null;
        if (next) current = { ...next.start };
      },
    },
    afterEdit() {},
    getTextAt,
    insertTextAtRaw,
    deleteTextAt: Reflect.get(InputHandler.prototype, 'deleteTextAt'),
    executeOperation: InputHandler.prototype.executeOperation,
    deleteSelection: Reflect.get(InputHandler.prototype, 'deleteSelection'),
    performPasteSelectedCells: () => false,
    onCopy(e: ClipboardEvent) { onCopy.call(host, e); },
    onCut(e: ClipboardEvent) { onCut.call(host, e); },
    onPaste(e: ClipboardEvent) { onPaste.call(host, e); },
  };
  return host;
}

const dummyClipEvent = (): ClipboardEvent => ({ preventDefault() {}, clipboardData: null } as unknown as ClipboardEvent);
const beginId = (id: number): string =>
  `<hp:ctrl><hp:fieldBegin id="${id}" type="CLICK_HERE" name="field${id}" editable="1" fieldid="${500 + id}"/></hp:ctrl>`;
const endId = (id: number): string =>
  `<hp:ctrl><hp:fieldEnd beginIDRef="${id}" fieldid="${500 + id}"/></hp:ctrl>`;
const floatingTable = (inner: string): string =>
  `<hp:tbl rowCnt="1" colCnt="1" borderFillIDRef="1"><hp:sz width="4000" widthRelTo="ABSOLUTE" height="1000" heightRelTo="ABSOLUTE" protect="0"/><hp:pos treatAsChar="0" affectLSpacing="0" flowWithText="1" allowOverlap="1" holdAnchorAndSO="0" vertRelTo="PARA" horzRelTo="PARA" vertAlign="TOP" horzAlign="LEFT" vertOffset="100" horzOffset="100"/><hp:tr><hp:tc borderFillIDRef="1"><hp:subList>${inner}</hp:subList><hp:cellAddr colAddr="0" rowAddr="0"/><hp:cellSpan colSpan="1" rowSpan="1"/><hp:cellSz width="4000" height="1000"/></hp:tc></hp:tr></hp:tbl>`;

test('R6 SNAP1-4 snapshot inverses refuse unrecorded replaceAll and setFieldValue mutations', () => {
  // SNAP1: paste snapshot -> undo -> replaceAll -> redo refused
  {
    const wasm = new WasmBridge(), history = new CommandHistory();
    try {
      wasm.loadDocument(fixture(para(`${begin}<hp:t>ABC</hp:t>${end}`) + para('<hp:t>X</hp:t>')));
      wasm.copySelection(0, 1, 0, 1, 1);
      const host = makeStudioHost(wasm, history, null);
      host.cursor.moveTo(position(0, 1));
      host.onPaste(dummyClipEvent());
      assert.ok(history.undo(wasm));
      wasm.replaceAll('ABC', 'NEW', true);
      const beforeRedo = wasm.exportHwpx();
      assert.equal(history.redo(wasm), null);
      assert.equal(history.canRedo(), false);
      assert.equal(wasm.getTextRange(0, 0, 0, 20), 'NEW');
      assert.deepEqual(wasm.exportHwpx(), beforeRedo);
      roundTrip(wasm);
    } finally { history.clear(wasm); wasm.dispose(); }
  }

  // SNAP2: paste snapshot -> setFieldValue -> undo refused
  {
    const wasm = new WasmBridge(), history = new CommandHistory();
    try {
      wasm.loadDocument(fixture(para(`${begin}<hp:t>ABC</hp:t>${end}`) + para('<hp:t>QQ</hp:t>') + para('<hp:t>X</hp:t>')));
      wasm.copySelection(0, 2, 0, 2, 1);
      const host = makeStudioHost(wasm, history, null);
      host.cursor.moveTo(position(1, 0));
      host.onPaste(dummyClipEvent());
      wasm.setFieldValue(501, 'NEW');
      const beforeUndo = wasm.exportHwpx();
      assert.equal(history.undo(wasm), null);
      assert.equal(history.canUndo(), false);
      assert.equal(wasm.getTextRange(0, 0, 0, 20), 'NEW');
      assert.deepEqual(wasm.exportHwpx(), beforeUndo);
      roundTrip(wasm);
    } finally { history.clear(wasm); wasm.dispose(); }
  }

  // SNAP3: IME-in-selection snapshot -> undo -> replaceAll -> redo refused
  {
    const wasm = new WasmBridge(), history = new CommandHistory();
    try {
      wasm.loadDocument(fixture(para(`${begin}<hp:t>AB</hp:t>${end}`)));
      const host = makeStudioHost(wasm, history, { start: position(0, 0), end: position(0, 1) });
      onCompositionStart.call(host);
      host.textarea.value = '가';
      onInput.call(host);
      host.textarea.value = '각';
      onInput.call(host);
      onCompositionEnd.call(host);
      assert.ok(history.undo(wasm));
      wasm.replaceAll('AB', 'NEW', true);
      const beforeRedo = wasm.exportHwpx();
      assert.equal(history.redo(wasm), null);
      assert.equal(history.canRedo(), false);
      assert.equal(wasm.getTextRange(0, 0, 0, 20), 'NEW');
      assert.deepEqual(wasm.exportHwpx(), beforeRedo);
      roundTrip(wasm);
    } finally { history.clear(wasm); wasm.dispose(); }
  }

  // SNAP4: cell paste snapshot -> undo -> setFieldValue -> redo refused
  {
    const wasm = new WasmBridge(), history = new CommandHistory();
    try {
      wasm.loadDocument(fixture(cell(para('<hp:t>CELL</hp:t>')) + para(`${begin}<hp:t>ABC</hp:t>${end}`) + para('<hp:t>X</hp:t>')));
      wasm.copySelection(0, 2, 0, 2, 1);
      const host = makeStudioHost(wasm, history, null);
      host.cursor.moveTo({ ...position(0, 0), parentParaIndex: 0, controlIndex: 0, cellIndex: 0, cellParaIndex: 0 });
      host.onPaste(dummyClipEvent());
      assert.ok(history.undo(wasm));
      wasm.setFieldValue(501, 'NEW');
      const beforeRedo = wasm.exportHwpx();
      assert.equal(history.redo(wasm), null);
      assert.equal(history.canRedo(), false);
      assert.equal(wasm.getTextRange(0, 1, 0, 20), 'NEW');
      assert.deepEqual(wasm.exportHwpx(), beforeRedo);
      roundTrip(wasm);
    } finally { history.clear(wasm); wasm.dispose(); }
  }
});

test('R6 boundary copies, cut-paste parity, CUT1/CUT2 and PARTITION obey half-open membership through WasmBridge', () => {
  const secretTable = cell(para(`${begin}<hp:t>SECRET</hp:t>${end}`));
  const leadingTablePara = secretTable.replace('</hp:tbl>', '</hp:tbl><hp:t>ZZ</hp:t>');

  // Boundary copy P0:0..P1:0 ending at start of table paragraph omits P1's table
  {
    const wasm = new WasmBridge();
    try {
      wasm.loadDocument(fixture(para('<hp:t>BEFORE</hp:t>') + leadingTablePara + para('<hp:t>TARGET</hp:t>')));
      wasm.copySelection(0, 0, 0, 1, 0);
      assert.equal(wasm.getClipboardText(), 'BEFORE\n');
      wasm.pasteInternal(0, 2, 0);
      const xml = readSectionXmlFast(wasm.exportHwpx());
      assert.equal(countXmlTag(xml, 'tbl'), 1);
      assert.equal(countXmlTag(xml, 'fieldBegin'), 1);
      roundTrip(wasm);
    } finally { wasm.dispose(); }
  }

  // Trailing-table right boundaries [0,2) and [1,2) on ZZ<tbl/> omit the trailing table at anchor 2
  for (const startOff of [0, 1]) {
    const wasm = new WasmBridge();
    try {
      wasm.loadDocument(fixture(para('<hp:t>ZZ</hp:t>') + secretTable + para('<hp:t>TARGET</hp:t>')));
      wasm.mergeParagraph(0, 1);
      wasm.copySelection(0, 0, startOff, 0, 2);
      wasm.pasteInternal(0, 1, 0);
      const xml = readSectionXmlFast(wasm.exportHwpx());
      assert.equal(countXmlTag(xml, 'tbl'), 1, `trailing startOff=${startOff}`);
      assert.equal(countXmlTag(xml, 'fieldBegin'), 1, `trailing startOff=${startOff}`);
    } finally { wasm.dispose(); }
  }

  // Leading-table parity witnesses [0,1) and [0,2) on <tbl/>ZZ: copy includes, cut/delete goes 1->0, one paste restores 1
  for (const endOff of [1, 2]) {
    const wasm = new WasmBridge(), history = new CommandHistory();
    try {
      wasm.loadDocument(fixture(leadingTablePara + para('<hp:t>TARGET</hp:t>')));
      const host = makeStudioHost(wasm, history, { start: position(0, 0), end: position(0, endOff) });
      host.onCut(dummyClipEvent());
      const cutXml = readSectionXmlFast(wasm.exportHwpx());
      assert.equal(countXmlTag(cutXml, 'tbl'), 0, `parity endOff=${endOff} after cut`);
      assert.equal(countXmlTag(cutXml, 'fieldBegin'), 0, `parity endOff=${endOff} after cut`);
      roundTrip(wasm);

      host.cursor.setSelection(null);
      host.cursor.moveTo(position(1, 0));
      host.onPaste(dummyClipEvent());
      const pasteXml = readSectionXmlFast(wasm.exportHwpx());
      assert.equal(countXmlTag(pasteXml, 'tbl'), 1, `parity endOff=${endOff} after paste`);
      assert.equal(countXmlTag(pasteXml, 'fieldBegin'), 1, `parity endOff=${endOff} after paste`);
      roundTrip(wasm);
    } finally { history.clear(wasm); wasm.dispose(); }
  }

  // CUT2: Studio cut of BEFORE + paragraph break ending at P1:0, then paste
  {
    const wasm = new WasmBridge(), history = new CommandHistory();
    try {
      wasm.loadDocument(fixture(para('<hp:t>BEFORE</hp:t>') + leadingTablePara + para('<hp:t>TARGET</hp:t>')));
      const host = makeStudioHost(wasm, history, { start: position(0, 0), end: position(1, 0) });
      host.onCut(dummyClipEvent());
      const cutXml = readSectionXmlFast(wasm.exportHwpx());
      assert.equal(countXmlTag(cutXml, 'tbl'), 1);
      assert.equal(countXmlTag(cutXml, 'fieldBegin'), 1);
      host.cursor.setSelection(null);
      host.cursor.moveTo(position(wasm.getParagraphCount(0) - 1, 0));
      host.onPaste(dummyClipEvent());
      const pasteXml = readSectionXmlFast(wasm.exportHwpx());
      assert.equal(countXmlTag(pasteXml, 'tbl'), 1);
      assert.equal(countXmlTag(pasteXml, 'fieldBegin'), 1);
      roundTrip(wasm);
    } finally { history.clear(wasm); wasm.dispose(); }
  }

  // PARTITION: adjacent disjoint selections P0:0..P1:0 and P1:0..P1:1 partition the table at P1:0
  {
    const wasm = new WasmBridge();
    try {
      wasm.loadDocument(fixture(para('<hp:t>BEFORE</hp:t>') + leadingTablePara + para('<hp:t>TARGET</hp:t>')));
      wasm.copySelection(0, 0, 0, 1, 0);
      wasm.pasteInternal(0, 2, 0);
      const destPara = wasm.getParagraphCount(0) - 1;
      wasm.copySelection(0, 1, 0, 1, 1);
      wasm.pasteInternal(0, destPara, 0);
      const xml = readSectionXmlFast(wasm.exportHwpx());
      assert.equal(countXmlTag(xml, 'tbl'), 2);
      assert.equal(countXmlTag(xml, 'fieldBegin'), 2);
      roundTrip(wasm);
    } finally { wasm.dispose(); }
  }
});

test('R6 MIX1/3/4/5/6 mixed command chains and FindDialog/HwpCtrl invalidation share one label model', () => {
  // MIX1: type X -> bold -> undo -> undo -> redo -> redo
  {
    const wasm = new WasmBridge(), history = new CommandHistory();
    try {
      wasm.loadDocument(fixture(para('<hp:t>AB</hp:t>')));
      history.execute(new InsertTextCommand(position(0, 2), 'X'), wasm);
      history.execute(new ApplyCharFormatCommand(position(0, 0), position(0, 3), { bold: true }), wasm);
      assert.equal(history.entryLabels().undo.length, 2);
      assert.ok(history.undo(wasm));
      assert.equal(wasm.getTextRange(0, 0, 0, 20), 'ABX');
      assert.ok(history.undo(wasm));
      assert.equal(wasm.getTextRange(0, 0, 0, 20), 'AB');
      assert.equal(history.canUndo(), false);
      assert.ok(history.redo(wasm));
      assert.ok(history.redo(wasm));
      assert.equal(wasm.getTextRange(0, 0, 0, 20), 'ABX');
      roundTrip(wasm);
    } finally { history.clear(wasm); wasm.dispose(); }
  }

  // MIX3: paste (snapshot) -> type -> italic -> undo x3
  {
    const wasm = new WasmBridge(), history = new CommandHistory();
    try {
      wasm.loadDocument(fixture(para('<hp:t>AB</hp:t>') + para('<hp:t>Q</hp:t>')));
      wasm.copySelection(0, 1, 0, 1, 1);
      const host = makeStudioHost(wasm, history, null);
      host.cursor.moveTo(position(0, 2));
      host.onPaste(dummyClipEvent());
      history.execute(new InsertTextCommand(position(0, 3), 'Y'), wasm);
      history.execute(new ApplyCharFormatCommand(position(0, 0), position(0, 4), { italic: true }), wasm);
      assert.ok(history.undo(wasm));
      assert.ok(history.undo(wasm));
      assert.ok(history.undo(wasm));
      assert.equal(wasm.getTextRange(0, 0, 0, 20), 'AB');
      assert.equal(history.canUndo(), false);
      roundTrip(wasm);
    } finally { history.clear(wasm); wasm.dispose(); }
  }

  // MIX4: type inside field -> underline -> undo x2 -> redo x2
  {
    const wasm = new WasmBridge(), history = new CommandHistory();
    try {
      wasm.loadDocument(fixture(para(`${begin}<hp:t>AB</hp:t>${end}`)));
      const original = roundTrip(wasm);
      history.execute(new InsertTextCommand(position(0, 1), 'X'), wasm);
      history.execute(new ApplyCharFormatCommand(position(0, 0), position(0, 3), { underline: true }), wasm);
      const formatted = roundTrip(wasm);
      assert.ok(history.undo(wasm));
      assert.ok(history.undo(wasm));
      assert.equal(wasm.getTextRange(0, 0, 0, 20), 'AB');
      assert.equal(roundTrip(wasm), original);
      assert.ok(history.redo(wasm));
      assert.ok(history.redo(wasm));
      assert.equal(roundTrip(wasm), formatted);
    } finally { history.clear(wasm); wasm.dispose(); }
  }

  // MIX5: type X -> bold -> type Y -> undo x3 via executeOperation
  {
    const wasm = new WasmBridge(), history = new CommandHistory();
    try {
      wasm.loadDocument(fixture(para('<hp:t>AB</hp:t>')));
      const host = makeStudioHost(wasm, history, null);
      host.executeOperation({ kind: 'command', command: new InsertTextCommand(position(0, 2), 'X') });
      host.executeOperation({ kind: 'command', command: new ApplyCharFormatCommand(position(0, 0), position(0, 3), { bold: true }) });
      host.executeOperation({ kind: 'command', command: new InsertTextCommand(position(0, 3), 'Y') });
      assert.ok(history.undo(wasm));
      assert.ok(history.undo(wasm));
      assert.ok(history.undo(wasm));
      assert.equal(wasm.getTextRange(0, 0, 0, 20), 'AB');
      assert.equal(history.canUndo(), false);
    } finally { history.clear(wasm); wasm.dispose(); }
  }

  // MIX6: type X -> MoveTableCommand -> undo x2 -> redo x2
  {
    const wasm = new WasmBridge(), history = new CommandHistory();
    try {
      wasm.loadDocument(fixture(para(`${floatingTable(para('<hp:t>CELL</hp:t>'))}<hp:t>AB</hp:t>`)));
      const origProps = wasm.getTableProperties(0, 0, 0);
      history.execute(new InsertTextCommand(position(0, 2), 'X'), wasm);
      history.execute(new MoveTableCommand(0, 0, 0, 150, 200, 0, 0), wasm);
      assert.ok(history.undo(wasm));
      assert.equal(wasm.getTableProperties(0, 0, 0).horzOffset, origProps.horzOffset);
      assert.ok(history.undo(wasm));
      assert.equal(wasm.getTextRange(0, 0, 0, 20), 'AB');
      assert.equal(history.canUndo(), false);
      assert.ok(history.redo(wasm));
      assert.ok(history.redo(wasm));
      assert.equal(wasm.getTextRange(0, 0, 0, 20), 'ABX');
      roundTrip(wasm);
    } finally { history.clear(wasm); wasm.dispose(); }
  }

  // HwpCtrl PutFieldText bumps state label and invalidates older inverses
  {
    const wasm = new WasmBridge(), history = new CommandHistory();
    try {
      const namedBegin = '<hp:ctrl><hp:fieldBegin id="1" type="CLICK_HERE" name="field" editable="1" fieldid="501"><hp:parameters cnt="1" name=""><hp:stringParam name="Name">field</hp:stringParam></hp:parameters></hp:fieldBegin></hp:ctrl>';
      wasm.loadDocument(fixture(para(`${namedBegin}<hp:t>ABC</hp:t>${end}`)));
      const fields = wasm.getFieldList();
      assert.equal(fields.length, 1);
      const fieldName = fields[0].name;
      history.execute(new InsertTextCommand(position(0, 1), 'Z'), wasm);
      const ctrl = new HwpCtrl((wasm as unknown as { doc: unknown }).doc);
      let callbackCount = 0;
      ctrl.onHistoryInvalidated(() => {
        callbackCount += 1;
      });
      const before = stateLabelOf(wasm);
      assert.equal(ctrl.PutFieldText(fieldName, 'FROM_CTRL'), true);
      assert.equal(callbackCount, 1);
      assert.notEqual(stateLabelOf(wasm), before);
      assert.equal(history.undo(wasm), null);
      assert.equal(history.canUndo(), false);
      assert.equal(wasm.getTextRange(0, 0, 0, 20), 'FROM_CTRL');
      roundTrip(wasm);
    } finally { history.clear(wasm); wasm.dispose(); }
  }
});

// C7: size-changing format steps reflow and the format inverse is an exact snapshot pair, so the
// 1536-step run needs more than the C6 30 s budget on a loaded host (same seeds, same invariants).
test('deterministic mixed-sequence property test preserves inverse, reachability and non-duplication invariants across seeds 0..63', { timeout: 120_000 }, () => {
  const SEED_START = 0;
  const SEED_END = 63;
  const STEPS_PER_SEED = 24;

  function mulberry32(seed: number): () => number {
    let state = seed >>> 0;
    return () => {
      state = (state + 0x6d2b79f5) >>> 0;
      let t = Math.imul(state ^ (state >>> 15), 1 | state);
      t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }

  const baseFixtureBytes = fixture(
    para('<hp:t>BEFORE</hp:t>')
    + para(`${floatingTable(para(`${beginId(1)}<hp:t>SECRET</hp:t>${endId(1)}`))}<hp:t>ZZ</hp:t>`)
    + para(`${beginId(2)}<hp:t>INIT</hp:t>${endId(2)}<hp:t>TAIL</hp:t>`)
    + para('<hp:t>TARGET</hp:t>'),
  );

  let totalSteps = 0;
  for (let seed = SEED_START; seed <= SEED_END; seed++) {
    const rng = mulberry32(seed);
    const pick = (n: number): number => Math.floor(rng() * n);
    const wasm = new WasmBridge();
    const history = new CommandHistory();

    try {
      wasm.loadDocument(baseFixtureBytes, `seed-${seed}.hwpx`);
      const bytesByLabel = new Map<string, Uint8Array>();
      const recordLiveState = () => {
        const label = stateLabelOf(wasm);
        assert.notEqual(label, null, `seed=${seed}: missing stateLabel`);
        bytesByLabel.set(label!, wasm.exportHwpx());
      };
      recordLiveState();

      let tokenCounter = 0;
      for (let step = 0; step < STEPS_PER_SEED; step++) {
        totalSteps += 1;
        const action = pick(10);

        if (action === 0 || action === 1) {
          // Recorded text insert in P0 or P3
          const targetPara = pick(2) === 0 ? 0 : Math.min(3, wasm.getParagraphCount(0) - 1);
          const len = wasm.getParagraphLength(0, targetPara);
          const off = len > 0 ? pick(len + 1) : 0;
          const ch = String.fromCharCode(65 + (tokenCounter++ % 20));
          history.execute(new InsertTextCommand(position(targetPara, off), ch, step * 1000), wasm);
          recordLiveState();
        } else if (action === 2) {
          // Recorded paste snapshot into P0 or last paragraph
          const lastPara = wasm.getParagraphCount(0) - 1;
          const srcLen = wasm.getParagraphLength(0, lastPara);
          if (srcLen > 0) {
            wasm.copySelection(0, lastPara, 0, lastPara, Math.min(2, srcLen));
            const host = makeStudioHost(wasm, history, null);
            host.cursor.moveTo(position(0, Math.min(1, wasm.getParagraphLength(0, 0))));
            host.onPaste(dummyClipEvent());
            recordLiveState();
          }
        } else if (action === 3) {
          // Recorded character format command. Size-changing formats reflow line segments,
          // so they also run on a later paragraph and across a paragraph boundary.
          const lastPara = wasm.getParagraphCount(0) - 1;
          const targetPara = pick(2) === 0 ? 0 : lastPara;
          const len = wasm.getParagraphLength(0, targetPara);
          if (len > 0) {
            const kind = pick(5);
            const props: Partial<CharProperties> = kind === 0 ? { bold: true } : kind === 1 ? { italic: true }
              : kind === 2 ? { underline: true } : { fontSize: [800, 1400, 2400][pick(3)] };
            const spanning = kind >= 3 && targetPara > 0 && pick(2) === 0;
            const start = spanning ? position(targetPara - 1, 0) : position(targetPara, 0);
            history.execute(new ApplyCharFormatCommand(start, position(targetPara, len), props), wasm);
            recordLiveState();
          }
        } else if (action === 4) {
          // Recorded table move command (find current paragraph holding the floating table)
          const pCount = wasm.getParagraphCount(0);
          let tablePara = -1;
          for (let p = 0; p < pCount; p++) {
            try {
              wasm.getTableProperties(0, p, 0);
              tablePara = p;
              break;
            } catch { /* not in p */ }
          }
          if (tablePara >= 0) {
            const dh = (pick(5) - 2) * 50 || 50;
            const dv = (pick(5) - 2) * 50 || 50;
            history.execute(new MoveTableCommand(0, tablePara, 0, dh, dv, tablePara, 0, step * 1000), wasm);
            recordLiveState();
          }
        } else if (action === 5) {
          // Unrecorded replaceAll
          const lastPara = wasm.getParagraphCount(0) - 1;
          const sample = wasm.getTextRange(0, lastPara, 0, 3);
          if (sample.length >= 2) {
            wasm.replaceAll(sample, `R${tokenCounter++}`, true);
            recordLiveState();
          }
        } else if (action === 6) {
          // Unrecorded setFieldValue on field 502
          wasm.setFieldValue(502, `V${seed}_${step}`);
          recordLiveState();
        } else if (action === 7) {
          // Undo invariant check (exact state restoration OR zero-byte refusal + reachability)
          const beforeBytes = wasm.exportHwpx();
          const liveBefore = stateLabelOf(wasm);
          const undoEntriesBefore = history.entryLabels().undo;
          const top = undoEntriesBefore.at(-1);
          const result = history.undo(wasm);
          const afterBytes = wasm.exportHwpx();
          const liveAfter = stateLabelOf(wasm);

          if (top && liveBefore === top.undoLabel) {
            assert.notEqual(result, null, `seed=${seed} step=${step}: matching undoLabel must apply`);
            assert.equal(liveAfter, top.redoLabel, `seed=${seed} step=${step}: undo must restore redoLabel`);
            const expectedBytes = bytesByLabel.get(top.redoLabel);
            assert.ok(expectedBytes, `seed=${seed} step=${step}: unknown redoLabel state`);
            assert.deepEqual(afterBytes, expectedBytes, `seed=${seed} step=${step}: undo bytes mismatch`);
          } else {
            assert.equal(result, null, `seed=${seed} step=${step}: mismatched undoLabel must be refused`);
            assert.deepEqual(afterBytes, beforeBytes, `seed=${seed} step=${step}: refused undo changed bytes`);
            const hadApplicableOlder = undoEntriesBefore.slice(0, -1).some(e => e.undoLabel === liveAfter);
            if (hadApplicableOlder) {
              assert.equal(history.entryLabels().undo.at(-1)?.undoLabel, liveAfter, `seed=${seed} step=${step}: older applicable undo entry became unreachable`);
            } else {
              assert.equal(history.canUndo(), false, `seed=${seed} step=${step}: refused undo left unreachable entries`);
            }
          }
        } else if (action === 8) {
          // Redo invariant check (exact state restoration OR zero-byte refusal + reachability)
          const beforeBytes = wasm.exportHwpx();
          const liveBefore = stateLabelOf(wasm);
          const redoEntriesBefore = history.entryLabels().redo;
          const top = redoEntriesBefore.at(-1);
          const result = history.redo(wasm);
          const afterBytes = wasm.exportHwpx();
          const liveAfter = stateLabelOf(wasm);

          if (top && liveBefore === top.redoLabel) {
            assert.notEqual(result, null, `seed=${seed} step=${step}: matching redoLabel must apply`);
            assert.equal(liveAfter, top.undoLabel, `seed=${seed} step=${step}: redo must restore undoLabel`);
            const expectedBytes = bytesByLabel.get(top.undoLabel);
            assert.ok(expectedBytes, `seed=${seed} step=${step}: unknown undoLabel state`);
            assert.deepEqual(afterBytes, expectedBytes, `seed=${seed} step=${step}: redo bytes mismatch`);
          } else {
            assert.equal(result, null, `seed=${seed} step=${step}: mismatched redoLabel must be refused`);
            assert.deepEqual(afterBytes, beforeBytes, `seed=${seed} step=${step}: refused redo changed bytes`);
            const hadApplicableOlder = redoEntriesBefore.slice(0, -1).some(e => e.redoLabel === liveAfter);
            if (hadApplicableOlder) {
              assert.equal(history.entryLabels().redo.at(-1)?.redoLabel, liveAfter, `seed=${seed} step=${step}: older applicable redo entry became unreachable`);
            } else {
              assert.equal(history.canRedo(), false, `seed=${seed} step=${step}: refused redo left unreachable entries`);
            }
          }
        } else {
          // Invariant 2 & 3 probe on current state:
          // (a) Refusal above an applicable older entry never blocks the older entry;
          // (b) Cut+paste and adjacent copies never duplicate controls or fields.
          if (history.entryLabels().undo.at(-1)?.undoLabel === stateLabelOf(wasm)) {
            const applicableLabel = stateLabelOf(wasm)!;
            // Record a synthetic stale entry whose undoLabel belongs to a separate WasmBridge epoch
            const staleBridge = new WasmBridge();
            try {
              staleBridge.loadDocument(baseFixtureBytes, 'stale.hwpx');
              const staleBefore = stateLabelOf(staleBridge)!;
              staleBridge.insertText(0, 0, 0, 'S');
              const dummyCmd: EditCommand = {
                type: 'staleProbe',
                timestamp: 0,
                execute: () => position(0, 0),
                undo: () => position(0, 0),
                mergeWith: () => null,
              };
              history.recordWithoutExecute(dummyCmd, staleBridge, staleBefore);
            } finally {
              staleBridge.dispose();
            }
            const snapBeforeRefusal = wasm.exportHwpx();
            // First undo refuses the stale top with zero byte change and truncates ONLY the stale suffix
            assert.equal(history.undo(wasm), null, `seed=${seed} step=${step}: stale top must refuse`);
            assert.deepEqual(wasm.exportHwpx(), snapBeforeRefusal);
            assert.equal(history.entryLabels().undo.at(-1)?.undoLabel, applicableLabel);
            // Immediately verify the older applicable entry applies and restores exact bytes!
            const olderTargetLabel = history.entryLabels().undo.at(-1)!.redoLabel;
            assert.notEqual(history.undo(wasm), null, `seed=${seed} step=${step}: older applicable entry must remain reachable`);
            assert.deepEqual(wasm.exportHwpx(), bytesByLabel.get(olderTargetLabel));
            assert.notEqual(history.redo(wasm), null);
          }

          // Cut+paste and adjacent-copy non-duplication check on a clone of current document bytes
          const currentBytes = wasm.exportHwpx();
          const baseXml = readSectionXmlFast(currentBytes);
          const origTables = countXmlTag(baseXml, 'tbl');
          const origFields = countXmlTag(baseXml, 'fieldBegin');

          const cutClone = new WasmBridge(), cutHistory = new CommandHistory();
          try {
            cutClone.loadDocument(currentBytes, 'cut-probe.hwpx');
            const pCount = cutClone.getParagraphCount(0);
            if (pCount >= 2) {
              const sel = pick(2) === 0
                ? { start: position(0, 0), end: position(1, 0) }
                : { start: position(1, 0), end: position(1, Math.min(1, Math.max(1, cutClone.getParagraphLength(0, 1)))) };
              const host = makeStudioHost(cutClone, cutHistory, sel);
              host.onCut(dummyClipEvent());
              host.cursor.setSelection(null);
              const destP = Math.max(0, cutClone.getParagraphCount(0) - 1);
              host.cursor.moveTo(position(destP, 0));
              host.onPaste(dummyClipEvent());
              const afterCutPasteXml = readSectionXmlFast(cutClone.exportHwpx());
              assert.equal(countXmlTag(afterCutPasteXml, 'tbl'), origTables, `seed=${seed} step=${step}: cut+paste duplicated or lost table`);
              assert.equal(countXmlTag(afterCutPasteXml, 'fieldBegin'), origFields, `seed=${seed} step=${step}: cut+paste duplicated or lost field`);
            }
          } finally { cutHistory.clear(cutClone); cutClone.dispose(); }

          const partClone = new WasmBridge();
          try {
            partClone.loadDocument(currentBytes, 'partition-probe.hwpx');
            if (partClone.getParagraphCount(0) >= 3) {
              const splitOff = Math.min(1, partClone.getParagraphLength(0, 1));
              partClone.copySelection(0, 0, 0, 1, 0);
              partClone.pasteInternal(0, partClone.getParagraphCount(0) - 1, 0);
              partClone.copySelection(0, 1, 0, 1, splitOff);
              partClone.pasteInternal(0, partClone.getParagraphCount(0) - 1, 0);

              const unionClone = new WasmBridge();
              try {
                unionClone.loadDocument(currentBytes, 'union-probe.hwpx');
                unionClone.copySelection(0, 0, 0, 1, splitOff);
                unionClone.pasteInternal(0, unionClone.getParagraphCount(0) - 1, 0);
                const partXml = readSectionXmlFast(partClone.exportHwpx());
                const unionXml = readSectionXmlFast(unionClone.exportHwpx());
                assert.equal(countXmlTag(partXml, 'tbl'), countXmlTag(unionXml, 'tbl'), `seed=${seed} step=${step}: adjacent copies failed to partition tables`);
                assert.equal(countXmlTag(partXml, 'fieldBegin'), countXmlTag(unionXml, 'fieldBegin'), `seed=${seed} step=${step}: adjacent copies duplicated fields`);
              } finally { unionClone.dispose(); }
            }
          } finally { partClone.dispose(); }
        }
      }
      roundTrip(wasm);
    } finally {
      history.clear(wasm);
      wasm.dispose();
    }
  }
  console.log(`PROPERTY_SEED_RANGE=${SEED_START}..${SEED_END} seeds=${SEED_END - SEED_START + 1} steps=${totalSteps} violations=0`);
});

test('G17 TypeScript AST guard enforces one command-inverse authority and private dual-label stacks', () => {
  const studioSrc = new URL('../src', import.meta.url).pathname;
  const tsFiles: string[] = [];
  function walkTs(dir: string): void {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const full = join(dir, entry.name);
      if (entry.isDirectory()) walkTs(full);
      else if (entry.name.endsWith('.ts')) tsFiles.push(full);
    }
  }
  walkTs(studioSrc);
  tsFiles.sort();

  const beginCallSites: string[] = [];
  const finishCallSites: string[] = [];
  let privateStacks = 0;
  let historyEntryLabels = 0;
  const runGateInverseCallers: string[] = [];

  for (const file of tsFiles) {
    const code = readFileSync(file, 'utf8');
    const sf = ts.createSourceFile(file, code, ts.ScriptTarget.Latest, true);
    let currentFn = '<top>';

    function visit(node: ts.Node): void {
      const prevFn = currentFn;
      if (ts.isFunctionDeclaration(node) && node.name) currentFn = node.name.text;
      else if (ts.isMethodDeclaration(node) && ts.isIdentifier(node.name)) currentFn = node.name.text;

      if (ts.isCallExpression(node)) {
        if (ts.isPropertyAccessExpression(node.expression)) {
          const prop = node.expression.name.text;
          if (prop === 'beginCommandInverse') beginCallSites.push(`${file}:${currentFn}`);
          if (prop === 'finishCommandInverse') finishCallSites.push(`${file}:${currentFn}`);
        } else if (ts.isIdentifier(node.expression) && node.expression.text === 'runGateInverse') {
          runGateInverseCallers.push(`${file}:${currentFn}`);
        }
      }

      if (file.endsWith('/engine/history.ts')) {
        if (ts.isInterfaceDeclaration(node) && node.name.text === 'HistoryEntry') {
          for (const member of node.members) {
            if (ts.isPropertySignature(member) && member.name && ts.isIdentifier(member.name)) {
              if (member.name.text === 'undoLabel' || member.name.text === 'redoLabel') historyEntryLabels += 1;
            }
          }
        }
        if (ts.isClassDeclaration(node) && node.name?.text === 'CommandHistory') {
          for (const member of node.members) {
            if (ts.isPropertyDeclaration(member) && ts.isIdentifier(member.name)) {
              if (member.name.text === 'undoStack' || member.name.text === 'redoStack') {
                const isPrivate = member.modifiers?.some(m => m.kind === ts.SyntaxKind.PrivateKeyword) ?? false;
                assert.equal(isPrivate, true, `${member.name.text} must be private on CommandHistory`);
                privateStacks += 1;
              }
            }
          }
        }
      }

      ts.forEachChild(node, visit);
      currentFn = prevFn;
    }
    visit(sf);
  }

  assert.equal(beginCallSites.length, 1, `expected 1 beginCommandInverse call site, got ${JSON.stringify(beginCallSites)}`);
  assert.equal(finishCallSites.length, 1, `expected 1 finishCommandInverse call site, got ${JSON.stringify(finishCallSites)}`);
  assert.ok(beginCallSites[0].endsWith('/engine/history.ts:runGateInverse'));
  assert.ok(finishCallSites[0].endsWith('/engine/history.ts:runGateInverse'));
  assert.equal(privateStacks, 2);
  assert.equal(historyEntryLabels, 2);
  assert.deepEqual(
    runGateInverseCallers.map(s => s.split(':').pop()).sort(),
    ['redo', 'undo'],
  );
  console.log(`G17_TS_GUARD inverse_callers=${beginCallSites.length} private_stacks=${privateStacks} dual_label_entry=${historyEntryLabels === 2}`);
});

// ─── C7: R7 A–E regressions through production commands and handlers ──────

const wideCell = (inner: string): string => para(`<hp:tbl rowCnt="1" colCnt="1" borderFillIDRef="1"><hp:tr><hp:tc borderFillIDRef="1"><hp:subList>${inner}</hp:subList><hp:cellAddr colAddr="0" rowAddr="0"/><hp:cellSpan colSpan="1" rowSpan="1"/><hp:cellSz width="30000" height="1000"/></hp:tc></hp:tr></hp:tbl>`);
const cellAt = (cellParaIndex: number, charOffset: number): DocumentPosition => ({
  ...position(0, charOffset), parentParaIndex: 0, controlIndex: 0, cellIndex: 0, cellParaIndex,
});

interface ObjectRef { sec: number; ppi: number; ci: number; type: 'shape'; origHorzOffset: number; origVertOffset: number }

/**
 * Two floating rectangles joined by a real connector, plus a typing target paragraph.
 * The HWPX writer does not serialize these drawing objects and exportHwp runs a
 * mutating adapter, so object tests compare `objectState`: every control's full
 * properties, every paragraph's text and the rendered page SVG.
 */
function connectedShapes(wasm: WasmBridge): { shapes: ObjectRef[]; link: { ppi: number; ci: number }; target: number } {
  wasm.loadDocument(fixture(para('<hp:t>ONE</hp:t>') + para('<hp:t>TWO</hp:t>') + para('<hp:t>LINK</hp:t>')
    + para(`${begin}<hp:t>AB</hp:t>${end}`)), 'c7-objects.hwpx');
  const insts: number[] = [];
  const shapes = [0, 1].map(p => {
    const r = wasm.createShapeControl({ sectionIdx: 0, paraIdx: p, charOffset: 0, width: 4000, height: 4000,
      horzOffset: 1000 + p * 19000, vertOffset: 1000, shapeType: 'rectangle', treatAsChar: false });
    const props = wasm.getShapeProperties(0, r.paraIdx, r.controlIdx) as { horzOffset: number; vertOffset: number; scInstId: number };
    insts.push(props.scInstId);
    return { sec: 0, ppi: r.paraIdx, ci: r.controlIdx, type: 'shape' as const, origHorzOffset: props.horzOffset, origVertOffset: props.vertOffset };
  });
  const link = wasm.createShapeControl({ sectionIdx: 0, paraIdx: 2, charOffset: 0, width: 15000, height: 1,
    horzOffset: 5000, vertOffset: 3000, shapeType: 'connector-straight',
    startSubjectID: insts[0], startSubjectIndex: 1, endSubjectID: insts[1], endSubjectIndex: 3 });
  return { shapes, link: { ppi: link.paraIdx, ci: link.controlIdx }, target: wasm.getParagraphCount(0) - 1 };
}

/** Side-effect-free full state of the object fixture (see `connectedShapes`). */
function objectState(wasm: WasmBridge): string {
  const paragraphs: unknown[] = [];
  for (let p = 0; p < wasm.getParagraphCount(0); p++) {
    let shape: unknown = null;
    try { shape = wasm.getShapeProperties(0, p, 0); } catch { /* text-only paragraph */ }
    paragraphs.push({ text: wasm.getTextRange(0, p, 0, wasm.getParagraphLength(0, p)), shape });
  }
  return JSON.stringify({ paragraphs, svg: wasm.renderPageSvg(0) });
}

function shapeBox(wasm: WasmBridge, ppi: number, ci: number): number[] {
  const p = wasm.getShapeProperties(0, ppi, ci) as { horzOffset: number; vertOffset: number; width: number; height: number };
  return [p.horzOffset, p.vertOffset, p.width, p.height];
}

test('C7 R7-A font-size undo and redo restore line segments, bytes and the reused label in body and cell', () => {
  for (const where of ['body', 'cell'] as const) {
    const wasm = new WasmBridge(), history = new CommandHistory();
    try {
      const content = para(`${begin}<hp:t>AB</hp:t>${end}`) + para('<hp:t>TAIL</hp:t>');
      wasm.loadDocument(fixture(where === 'body' ? content : wideCell(content)), `r7a-${where}.hwpx`);
      const at = (p: number, off: number): DocumentPosition => (where === 'body' ? position(p, off) : cellAt(p, off));
      history.execute(new InsertTextCommand(at(0, 1), 'X', 1000), wasm);
      const typed = wasm.exportHwpx(), typedLabel = stateLabelOf(wasm);
      history.execute(new ApplyCharFormatCommand(at(0, 0), at(1, 2), { fontSize: 3600 }), wasm);
      const formatted = wasm.exportHwpx(), formattedLabel = stateLabelOf(wasm);
      assert.notDeepEqual(formatted, typed, `${where}: the format changed the document`);
      assert.ok(history.undo(wasm));
      assert.equal(stateLabelOf(wasm), typedLabel);
      assert.deepEqual(wasm.exportHwpx(), typed, `${where}: undo reused a label for different bytes`);
      assert.ok(history.redo(wasm));
      assert.equal(stateLabelOf(wasm), formattedLabel);
      assert.deepEqual(wasm.exportHwpx(), formatted, `${where}: redo reused a label for different bytes`);
      assert.ok(history.undo(wasm));
      assert.ok(history.undo(wasm));
      assert.equal(history.canUndo(), false);
      roundTrip(wasm);
    } finally { history.clear(wasm); wasm.dispose(); }
  }
});

test('C7 R7-B a failure after a real inverse step leaves bytes, label, history and native handles unchanged', () => {
  for (const step of ['restoreSnapshot', 'undoEdit'] as const) {
    for (const failure of ['refusal', 'error'] as const) {
      const name = `${step}/${failure}`;
      const wasm = new WasmBridge(), history = new CommandHistory();
      try {
        wasm.loadDocument(fixture(para('<hp:t>AA</hp:t>') + para('<hp:t>BB</hp:t>')), 'r7b.hwpx');
        history.execute(new InsertTextCommand(position(0, 0), 'X', 1000), wasm);
        const typed = wasm.exportHwpx();
        // restoreSnapshot: the top entry is a two-paragraph format; undoEdit: the top entry is the typing.
        if (step === 'restoreSnapshot') {
          history.execute(new ApplyCharFormatCommand(position(0, 0), position(1, 2), { bold: true }), wasm);
        }
        const before = wasm.exportHwpx(), label = stateLabelOf(wasm);
        const entries = history.entryLabels(), live = wasm.liveEditCount();
        const bridge = wasm as unknown as Record<string, (...args: unknown[]) => unknown>;
        const real = bridge[step].bind(wasm);
        let calls = 0, mutated = false;
        bridge[step] = (...args: unknown[]) => {
          calls++;
          const result = real(...args);
          mutated = stateLabelOf(wasm) !== label;
          void result;
          if (failure === 'refusal') throw new InvalidEditHandleError('INVALID_EDIT_HANDLE: injected failure after a real inverse step');
          throw new Error('injected failure after a real inverse step');
        };
        try {
          if (failure === 'refusal') assert.equal(history.undo(wasm), null, name);
          else assert.throws(() => history.undo(wasm), /injected failure/, name);
        } finally { delete bridge[step]; }
        assert.equal(calls, 1, `${name}: the injected step ran`);
        assert.equal(mutated, true, `${name}: the real step mutated before the failure`);
        assert.deepEqual(wasm.exportHwpx(), before, `${name}: bytes changed`);
        assert.equal(stateLabelOf(wasm), label, `${name}: label changed`);
        assert.deepEqual(history.entryLabels(), entries, `${name}: history entries changed`);
        assert.equal(wasm.liveEditCount(), live, `${name}: native handles changed`);
        // The refused entry and every older entry still apply exactly.
        assert.ok(history.undo(wasm), name);
        if (step === 'restoreSnapshot') {
          assert.deepEqual(wasm.exportHwpx(), typed, name);
          assert.ok(history.undo(wasm), name);
        }
        assert.equal(wasm.getTextRange(0, 0, 0, 2), 'AA', name);
        assert.equal(history.canUndo(), false, name);
        assert.ok(history.redo(wasm), name);
        if (step === 'restoreSnapshot') assert.ok(history.redo(wasm), name);
        assert.deepEqual(wasm.exportHwpx(), before, name);
        roundTrip(wasm);
      } finally { history.clear(wasm); wasm.dispose(); }
    }
  }
});

test('C7 R7-C multi-object drag is one exact entry with the true label pair and connector side effects', () => {
  const wasm = new WasmBridge(), history = new CommandHistory();
  try {
    const { shapes, link, target } = connectedShapes(wasm);
    history.execute(new InsertTextCommand(position(target, 0), 'X', 1000), wasm);
    const typed = objectState(wasm), typedLabel = stateLabelOf(wasm);
    const linkBefore = shapeBox(wasm, link.ppi, link.ci);
    const sc = { clientWidth: 100, getBoundingClientRect: () => ({ left: 0, top: 0 }) };
    const host = {
      wasm, history, dragRafId: 0, isPictureMoveDragging: true,
      pictureMoveState: { ref: shapes[0], lastPageX: 0, lastPageY: 0, pageIndex: 0, totalDeltaH: 0, totalDeltaV: 0,
        multiRefs: shapes, beforeLabel: typedLabel },
      container: { style: { cursor: 'move' }, querySelector: () => sc },
      viewportManager: { getZoom: () => 1 },
      virtualScroll: { getPageOffset: () => 0, getPageWidth: () => 100, getPageLeft: () => 0 },
      eventBus: { emit() {} }, renderPictureObjectSelection() {},
    };
    for (let n = 1; n <= 3; n++) updatePictureMoveDrag.call(host, { clientX: n * 10, clientY: n * 4 } as MouseEvent);
    finishPictureMoveDrag.call(host);
    const dragged = objectState(wasm), draggedLabel = stateLabelOf(wasm);
    for (const s of shapes) {
      assert.equal(wasm.getShapeProperties(0, s.ppi, s.ci).horzOffset, s.origHorzOffset + 30 * 75, 'every selected object moved');
    }
    assert.notDeepEqual(shapeBox(wasm, link.ppi, link.ci), linkBefore, 'the connector followed the objects');
    assert.deepEqual(history.entryLabels().undo, [
      { type: 'insertText', undoLabel: typedLabel, redoLabel: history.entryLabels().undo[0].redoLabel },
      { type: 'snapshot:moveObjects', undoLabel: draggedLabel, redoLabel: typedLabel },
    ], 'one drag entry carrying the true pre/post labels');

    assert.ok(history.undo(wasm));
    assert.deepEqual(objectState(wasm), typed, 'one undo restores every object and the connector');
    assert.equal(stateLabelOf(wasm), typedLabel);
    assert.deepEqual(shapeBox(wasm, link.ppi, link.ci), linkBefore);
    assert.ok(history.undo(wasm), 'the older typing is still reachable');
    assert.equal(wasm.getTextRange(0, target, 0, 2), 'AB');
    assert.equal(history.undo(wasm), null);
    assert.ok(history.redo(wasm));
    assert.ok(history.redo(wasm));
    assert.deepEqual(objectState(wasm), dragged);
    assert.equal(stateLabelOf(wasm), draggedLabel);
  } finally { history.clear(wasm); wasm.dispose(); }
});

test('C7 keyboard nudge of a multi-object selection is one exact entry including connectors', () => {
  const wasm = new WasmBridge(), history = new CommandHistory();
  try {
    const { shapes, link, target } = connectedShapes(wasm);
    history.execute(new InsertTextCommand(position(target, 0), 'X', 1000), wasm);
    const typed = objectState(wasm), typedLabel = stateLabelOf(wasm);
    const linkBefore = shapeBox(wasm, link.ppi, link.ci);
    const refs = shapes.map(({ sec, ppi, ci, type }) => ({ sec, ppi, ci, type }));
    const host = {
      wasm, history, gridStepMm: 1,
      cursor: { getSelectedPictureRefs: () => refs, getSelectedPictureRef: () => refs[0] },
      eventBus: { emit() {} }, renderPictureObjectSelection() {},
      executeOperation: InputHandler.prototype.executeOperation,
    };
    moveSelectedPicture.call(host, 'ArrowRight');
    const nudged = objectState(wasm), nudgedLabel = stateLabelOf(wasm);
    const step = Math.round(7200 / 25.4);
    for (const s of shapes) assert.equal(wasm.getShapeProperties(0, s.ppi, s.ci).horzOffset, s.origHorzOffset + step);
    assert.notDeepEqual(shapeBox(wasm, link.ppi, link.ci), linkBefore, 'the connector followed the objects');
    const undo = history.entryLabels().undo;
    assert.equal(undo.length, 2);
    assert.deepEqual(undo[1], { type: 'snapshot:moveObjects', undoLabel: nudgedLabel, redoLabel: typedLabel });
    assert.ok(history.undo(wasm));
    assert.deepEqual(objectState(wasm), typed);
    assert.equal(stateLabelOf(wasm), typedLabel);
    assert.ok(history.undo(wasm));
    assert.equal(history.canUndo(), false);
    assert.ok(history.redo(wasm));
    assert.ok(history.redo(wasm));
    assert.deepEqual(objectState(wasm), nudged);
  } finally { history.clear(wasm); wasm.dispose(); }
});

test('C7 R7-D a refusal reconciles both stacks against the live label', () => {
  const wasm = new WasmBridge(), history = new CommandHistory();
  try {
    wasm.loadDocument(fixture(para('<hp:t>AB</hp:t>')), 'r7d.hwpx');
    history.execute(new InsertTextCommand(position(0, 0), 'X', 1000), wasm);
    history.execute(new InsertTextCommand(position(0, 3), 'Y', 2000), wasm);
    assert.ok(history.undo(wasm));
    assert.equal(history.canRedo(), true);
    wasm.replaceAll('AB', 'NEW', true);
    const afterUndoSide = wasm.exportHwpx();
    assert.equal(history.undo(wasm), null);
    assert.deepEqual(wasm.exportHwpx(), afterUndoSide, 'refused undo changed bytes');
    assert.equal(history.canUndo(), false);
    assert.equal(history.canRedo(), false, 'a phantom redo survived the undo refusal');

    history.execute(new InsertTextCommand(position(0, 0), 'P', 3000), wasm);
    history.execute(new InsertTextCommand(position(0, 1), 'Q', 4000), wasm);
    assert.ok(history.undo(wasm));
    assert.equal(history.canUndo(), true);
    wasm.replaceAll('NEW', 'OUT', true);
    const afterRedoSide = wasm.exportHwpx();
    assert.equal(history.redo(wasm), null);
    assert.deepEqual(wasm.exportHwpx(), afterRedoSide, 'refused redo changed bytes');
    assert.equal(history.canRedo(), false);
    assert.equal(history.canUndo(), false, 'a phantom undo survived the redo refusal');

    history.execute(new InsertTextCommand(position(0, 0), 'Z', 5000), wasm);
    assert.ok(history.undo(wasm));
    assert.deepEqual(wasm.exportHwpx(), afterRedoSide);
    roundTrip(wasm);
  } finally { history.clear(wasm); wasm.dispose(); }
});

test('C7 R7-E an empty field at the included end boundary is copied whole and undone exactly', () => {
  const wasm = new WasmBridge(), history = new CommandHistory();
  try {
    wasm.loadDocument(fixture(para(`${beginId(1)}<hp:t>A</hp:t>${endId(1)}${beginId(2)}${endId(2)}`)
      + para('<hp:t>DEST</hp:t>')), 'r7e.hwpx');
    const original = wasm.exportHwpx();
    assert.equal(countXmlTag(readSectionXmlFast(original), 'fieldBegin'), 2);
    wasm.copySelection(0, 0, 0, 0, 1);
    history.execute(new SnapshotCommand('paste', position(1, 0), position(1, 1), w => {
      w.pasteInternal(0, 1, 0);
      return position(1, 1);
    }), wasm);
    assert.equal(countXmlTag(readSectionXmlFast(wasm.exportHwpx()), 'fieldBegin'), 4,
      'D6: a field with b == e == end belongs to the selection');
    roundTrip(wasm);
    assert.ok(history.undo(wasm));
    assert.deepEqual(wasm.exportHwpx(), original);
  } finally { history.clear(wasm); wasm.dispose(); }
});

test('C7 discarding a snapshot command after a reload leaves the reloaded document snapshots intact', () => {
  const wasm = new WasmBridge(), stale = new CommandHistory(), fresh = new CommandHistory();
  try {
    wasm.loadDocument(fixture(para('<hp:t>AB</hp:t>')), 'old.hwpx');
    stale.execute(new SnapshotCommand('insert', position(0, 0), position(0, 1), w => {
      w.insertText(0, 0, 0, 'S');
      return position(0, 1);
    }), wasm);
    wasm.loadDocument(fixture(para('<hp:t>CD</hp:t>')), 'new.hwpx');
    fresh.execute(new SnapshotCommand('insert', position(0, 0), position(0, 1), w => {
      w.insertText(0, 0, 0, 'T');
      return position(0, 1);
    }), wasm);
    stale.clear(wasm);
    assert.ok(fresh.undo(wasm), 'a stale discard deleted the reloaded document snapshot');
    assert.equal(wasm.getTextRange(0, 0, 0, 2), 'CD');
    assert.ok(fresh.redo(wasm));
    assert.equal(wasm.getTextRange(0, 0, 0, 3), 'TCD');
  } finally { stale.clear(); fresh.clear(wasm); wasm.dispose(); }
});

// ─── C8: R8 A–C regressions (snapshot retention, token rollback, gesture pairing) ──────

const r8Field = (): Uint8Array => fixture(para(`${begin}<hp:t>AB</hp:t>${end}`));
type SnapshotIds = { beforeId: number | null; afterId: number | null };
const captureIds = (command: ApplyCharFormatCommand): SnapshotIds => {
  const capture = (command as unknown as { capture: SnapshotIds }).capture;
  return { beforeId: capture.beforeId, afterId: capture.afterId };
};
const rawSnapshotLabels = (wasm: WasmBridge, id: number): string | undefined =>
  (wasm as unknown as { doc: { snapshotLabels(id: number): string | undefined } }).doc.snapshotLabels(id);
const keyEvent = (key: string, extra: Partial<KeyboardEvent> = {}): { event: KeyboardEvent; prevented: () => number } => {
  let prevented = 0;
  const event = { key, code: key, ctrlKey: false, metaKey: false, shiftKey: false, altKey: false,
    isComposing: false, keyCode: 0, preventDefault() { prevented++; }, stopPropagation() {}, ...extra };
  return { event: event as unknown as KeyboardEvent, prevented: () => prevented };
};
const stateNumber = (label: string): number => Number(label.split(':')[1]);

test('C8 R8-A 51 formats keep all 52 entries undoable to AB and a full MAX_HISTORY_SIZE of snapshot entries stays complete', () => {
  const wasm = new WasmBridge(), history = new CommandHistory();
  try {
    wasm.loadDocument(r8Field(), 'r8a.hwpx');
    history.execute(new InsertTextCommand(position(0, 0), 'X', 1000), wasm);
    for (let i = 0; i < 51; i++) {
      history.execute(new ApplyCharFormatCommand(position(0, 0), position(0, 1), { fontSize: i % 2 === 0 ? 1200 : 1400 }), wasm);
    }
    let undone = 0;
    while (undone < 60 && history.undo(wasm) !== null) undone++;
    assert.equal(undone, 52, '51 formatting commands and the older typing all stay undoable');
    assert.equal(wasm.getTextRange(0, 0, 0, 3), 'AB');
    assert.equal(history.canUndo(), false);
    let redone = 0;
    while (redone < 60 && history.redo(wasm) !== null) redone++;
    assert.equal(redone, 52);
    assert.equal(wasm.getTextRange(0, 0, 0, 3), 'XAB');
  } finally { history.clear(wasm); wasm.dispose(); }

  const full = new WasmBridge(), fullHistory = new CommandHistory();
  try {
    full.loadDocument(r8Field(), 'r8a-full.hwpx');
    const ids: SnapshotIds[] = [];
    for (let i = 0; i <= MAX_HISTORY_SIZE; i++) {
      const command = new ApplyCharFormatCommand(position(0, 0), position(0, 1), { fontSize: i % 2 === 0 ? 1200 : 1400 });
      fullHistory.execute(command, full);
      ids.push(captureIds(command));
    }
    assert.equal(fullHistory.entryLabels().undo.length, MAX_HISTORY_SIZE);
    const [evicted, oldest] = ids;
    assert.equal(rawSnapshotLabels(full, evicted.beforeId ?? -1), undefined, 'history eviction released the oldest pair');
    assert.equal(rawSnapshotLabels(full, evicted.afterId ?? -1), undefined);
    assert.notEqual(rawSnapshotLabels(full, oldest.beforeId ?? -1), undefined, 'the oldest retained entry kept its pair');
    assert.notEqual(rawSnapshotLabels(full, oldest.afterId ?? -1), undefined);
    let undoneFull = 0;
    while (undoneFull <= MAX_HISTORY_SIZE && fullHistory.undo(full) !== null) undoneFull++;
    assert.equal(undoneFull, MAX_HISTORY_SIZE, 'every retained snapshot entry undoes');
    assert.equal(fullHistory.canUndo(), false);
  } finally { fullHistory.clear(full); full.dispose(); }
});

test('C8 R8-A a permanently missing native inverse drops the dead entry and the entries behind it but keeps redo', () => {
  for (const kind of ['snapshot', 'text'] as const) {
    const wasm = new WasmBridge(), history = new CommandHistory();
    try {
      wasm.loadDocument(r8Field(), `r8a-missing-${kind}.hwpx`);
      history.execute(new InsertTextCommand(position(0, 0), 'X', 1000), wasm);
      const bold = new ApplyCharFormatCommand(position(0, 0), position(0, 1), { bold: true });
      history.execute(bold, wasm);
      const typing = new InsertTextCommand(position(0, 1), 'Y', 5000);
      if (kind === 'text') history.execute(typing, wasm);
      history.execute(new ApplyCharFormatCommand(position(0, 0), position(0, 1), { italic: true }), wasm);
      assert.ok(history.undo(wasm), 'the italic entry moves to the redo stack');
      const redoBefore = JSON.stringify(history.entryLabels().redo);

      if (kind === 'snapshot') wasm.discardSnapshot(captureIds(bold).beforeId ?? -1);
      else wasm.discardEdit((typing as unknown as { inverse: { token: string } }).inverse.token);
      const bytes = wasm.exportHwpx(), label = stateLabelOf(wasm);
      assert.equal(history.undo(wasm), null, `${kind}: the missing native inverse refuses`);
      assert.deepEqual(wasm.exportHwpx(), bytes, `${kind}: the refusal changed bytes`);
      assert.equal(stateLabelOf(wasm), label);
      assert.equal(history.canUndo(), false, `${kind}: the dead entry and the older entries it blocks are dropped`);
      assert.equal(history.undo(wasm), null);
      assert.equal(JSON.stringify(history.entryLabels().redo), redoBefore, `${kind}: the valid redo side is kept`);
      assert.ok(history.redo(wasm), `${kind}: the kept redo still applies`);
      assert.ok(history.undo(wasm), `${kind}: and undoes again`);
      assert.equal(history.canUndo(), false);
      roundTrip(wasm);
    } finally { history.clear(wasm); wasm.dispose(); }
  }
});

test('C8 R8-B a finishCommandInverse fault after a real text inverse rolls back both sides and the retry succeeds', () => {
  for (const direction of ['undo', 'redo'] as const) {
    const wasm = new WasmBridge(), history = new CommandHistory();
    try {
      wasm.loadDocument(fixture(para('<hp:t>AB</hp:t>')), `r8b-${direction}.hwpx`);
      history.execute(new InsertTextCommand(position(0, 0), 'X', 1000), wasm);
      if (direction === 'redo') assert.ok(history.undo(wasm));
      const before = wasm.exportHwpx(), label = stateLabelOf(wasm);
      const entries = JSON.stringify(history.entryLabels()), handles = wasm.liveEditCount();
      const raw = (wasm as unknown as { doc: Record<string, unknown> }).doc;
      let injected = 0;
      raw.finishCommandInverse = () => {
        injected++;
        throw new InvalidEditHandleError('INVALID_EDIT_HANDLE: injected after the text inverse returned its reciprocal token');
      };
      let refused: DocumentPosition | null;
      try { refused = direction === 'undo' ? history.undo(wasm) : history.redo(wasm); }
      finally { delete raw.finishCommandInverse; }
      assert.equal(injected, 1, `${direction}: the real inverse step ran before the injected finish fault`);
      assert.equal(refused, null);
      assert.deepEqual(wasm.exportHwpx(), before, `${direction}: the rolled-back scope changed bytes`);
      assert.equal(stateLabelOf(wasm), label);
      assert.equal(JSON.stringify(history.entryLabels()), entries, `${direction}: a transient rollback keeps the entry`);
      assert.equal(wasm.liveEditCount(), handles);
      const retried = direction === 'undo' ? history.undo(wasm) : history.redo(wasm);
      assert.ok(retried, `${direction}: the retry after the rolled-back scope succeeds`);
      assert.equal(wasm.getTextRange(0, 0, 0, 3), direction === 'undo' ? 'AB' : 'XAB');
      assert.ok(direction === 'undo' ? history.redo(wasm) : history.undo(wasm), `${direction}: the opposite direction still works`);
      assert.equal(wasm.getTextRange(0, 0, 0, 3), direction === 'undo' ? 'XAB' : 'AB');
      roundTrip(wasm);
    } finally { history.clear(wasm); wasm.dispose(); }
  }
});

test('C8 R8-C an open object drag refuses interleaved nudges and keys, then drag and nudge record sequential entries', () => {
  const wasm = new WasmBridge(), history = new CommandHistory();
  try {
    const { shapes, target } = connectedShapes(wasm);
    history.execute(new InsertTextCommand(position(target, 0), 'X', 1000), wasm);
    const typed = objectState(wasm), typedLabel = stateLabelOf(wasm) ?? '';
    const refs = shapes.map(({ sec, ppi, ci, type }) => ({ sec, ppi, ci, type }));
    const sc = { clientWidth: 100, getBoundingClientRect: () => ({ left: 0, top: 0 }) };
    const host = {
      active: true, wasm, history, gridStepMm: 1, dragRafId: 0, isPictureMoveDragging: true,
      pictureMoveState: { ref: shapes[0], lastPageX: 0, lastPageY: 0, pageIndex: 0, totalDeltaH: 0, totalDeltaV: 0,
        multiRefs: shapes, beforeLabel: typedLabel } as Record<string, unknown> | null,
      cursor: { getSelectedPictureRefs: () => refs, getSelectedPictureRef: () => refs[0] },
      container: { style: { cursor: 'move' }, querySelector: () => sc },
      viewportManager: { getZoom: () => 1 },
      virtualScroll: { getPageOffset: () => 0, getPageWidth: () => 100, getPageLeft: () => 0 },
      eventBus: { emit() {} }, renderPictureObjectSelection() {},
      executeOperation: InputHandler.prototype.executeOperation,
    };
    updatePictureMoveDrag.call(host, { clientX: 10, clientY: 0 } as MouseEvent);
    const midDrag = objectState(wasm);
    moveSelectedPicture.call(host, 'ArrowRight');
    const keys = [keyEvent('ArrowRight'), keyEvent('z', { ctrlKey: true }), keyEvent('Delete')];
    for (const k of keys) onKeyDown.call(host, k.event);
    assert.deepEqual(keys.map(k => k.prevented()), [1, 1, 1], 'every key is consumed while the drag is open');
    assert.deepEqual(objectState(wasm), midDrag, 'no nudge or key mutated the document inside the open drag');
    assert.deepEqual(history.entryLabels().undo.map(e => e.type), ['insertText'], 'no key recorded or discarded history');
    updatePictureMoveDrag.call(host, { clientX: 20, clientY: 0 } as MouseEvent);
    finishPictureMoveDrag.call(host);
    const draggedLabel = stateLabelOf(wasm) ?? '';
    moveSelectedPicture.call(host, 'ArrowRight');
    const nudgedLabel = stateLabelOf(wasm) ?? '';
    const step = Math.round(7200 / 25.4);
    for (const s of shapes) assert.equal(wasm.getShapeProperties(0, s.ppi, s.ci).horzOffset, s.origHorzOffset + 20 * 75 + step);
    const undo = history.entryLabels().undo;
    assert.deepEqual(undo.map(e => e.type), ['insertText', 'snapshot:moveObjects', 'snapshot:moveObjects']);
    assert.deepEqual(undo.slice(1).map(e => [e.redoLabel, e.undoLabel]), [[typedLabel, draggedLabel], [draggedLabel, nudgedLabel]],
      'drag then nudge: sequential, non-overlapping label pairs');
    for (const e of undo) assert.ok(stateNumber(e.redoLabel) < stateNumber(e.undoLabel), 'monotonic label pair');
    assert.ok(history.undo(wasm));
    for (const s of shapes) assert.equal(wasm.getShapeProperties(0, s.ppi, s.ci).horzOffset, s.origHorzOffset + 20 * 75);
    assert.ok(history.undo(wasm));
    assert.deepEqual(objectState(wasm), typed, 'the drag undo restores the exact pre-drag objects');
    assert.ok(history.undo(wasm));
    assert.equal(wasm.getTextRange(0, target, 0, 2), 'AB');
    for (const s of shapes) assert.equal(wasm.getShapeProperties(0, s.ppi, s.ci).horzOffset, s.origHorzOffset);
    assert.equal(history.canUndo(), false);
  } finally { history.clear(wasm); wasm.dispose(); }
});

test('C8 R8-C an open table drag refuses interleaved nudges and keys, then drag and nudge record sequential entries', () => {
  const wasm = new WasmBridge(), history = new CommandHistory();
  try {
    wasm.loadDocument(fixture(cell(para('<hp:t>T</hp:t>')) + para(`${begin}<hp:t>AB</hp:t>${end}`)), 'r8c-table.hwpx');
    const original = wasm.getTableProperties(0, 0, 0);
    history.execute(new InsertTextCommand(position(1, 0), 'X', 1000), wasm);
    const typedLabel = stateLabelOf(wasm) ?? '';
    let selected = { sec: 0, ppi: 0, ci: 0 };
    const sc = { clientWidth: 100, getBoundingClientRect: () => ({ left: 0, top: 0 }) };
    const host = {
      active: true, wasm, history, gridStepMm: 1, dragRafId: 0, isMoveDragging: true,
      moveDragState: { tableRef: { ...selected }, startPpi: 0, lastPageX: 0, lastPageY: 0, pageIndex: 0,
        totalDeltaH: 0, totalDeltaV: 0, beforeLabel: typedLabel } as Record<string, unknown> | null,
      cursor: {
        getSelectedTableRef: () => ({ ...selected }),
        updateSelectedTableRef(sec: number, ppi: number, ci: number) { selected = { sec, ppi, ci }; },
      },
      container: { style: { cursor: 'move' }, querySelector: () => sc },
      viewportManager: { getZoom: () => 1 },
      virtualScroll: { getPageOffset: () => 0, getPageWidth: () => 100, getPageLeft: () => 0 },
      eventBus: { emit() {} }, renderTableObjectSelection() {},
      executeOperation: InputHandler.prototype.executeOperation,
    };
    updateMoveDrag.call(host, { clientX: 10, clientY: 0 } as MouseEvent);
    const midDrag = wasm.exportHwpx(), midLabel = stateLabelOf(wasm);
    moveSelectedTable.call(host, 'ArrowRight');
    const keys = [keyEvent('ArrowRight'), keyEvent('z', { ctrlKey: true })];
    for (const k of keys) onKeyDown.call(host, k.event);
    assert.deepEqual(keys.map(k => k.prevented()), [1, 1]);
    assert.deepEqual(wasm.exportHwpx(), midDrag, 'no nudge or key mutated the table inside the open drag');
    assert.equal(stateLabelOf(wasm), midLabel);
    assert.deepEqual(history.entryLabels().undo.map(e => e.type), ['insertText']);
    finishMoveDrag.call(host);
    const draggedLabel = stateLabelOf(wasm) ?? '';
    assert.deepEqual(history.entryLabels().undo.slice(1).map(e => [e.type, e.redoLabel, e.undoLabel]),
      [['moveTable', typedLabel, draggedLabel]], 'the drag closes as one entry starting at the typing label');
    moveSelectedTable.call(host, 'ArrowRight');
    const nudgedLabel = stateLabelOf(wasm) ?? '';
    // A later nudge of the same table continues the drag entry through MoveTableCommand.mergeWith;
    // the merged pair still starts where the typing ends and ends at the live label.
    const undo = history.entryLabels().undo;
    assert.deepEqual(undo.map(e => [e.type, e.redoLabel, e.undoLabel]).slice(1),
      [['moveTable', typedLabel, nudgedLabel]], 'table drag then nudge: sequential, non-overlapping label pairs');
    assert.equal(undo[0].undoLabel, typedLabel);
    assert.ok(stateNumber(draggedLabel) < stateNumber(nudgedLabel));
    assert.ok(history.undo(wasm));
    const restored = wasm.getTableProperties(0, selected.ppi, selected.ci);
    assert.equal(restored.horzOffset, original.horzOffset);
    assert.equal(restored.vertOffset, original.vertOffset);
    assert.ok(history.undo(wasm));
    assert.equal(wasm.getTextRange(0, 1, 0, 2), 'AB');
    assert.equal(history.canUndo(), false);
    roundTrip(wasm);
  } finally { history.clear(wasm); wasm.dispose(); }
});

test('C8 R8-C overlapping gesture recorders keep their own snapshot ids and labels and a forged pair is not recorded', () => {
  const wasm = new WasmBridge(), history = new CommandHistory();
  try {
    const { shapes } = connectedShapes(wasm);
    const outer = new ObjectGestureRecorder('moveObjects', position(0, 0));
    const inner = new ObjectGestureRecorder('moveObjects', position(0, 0));
    const l0 = stateLabelOf(wasm) ?? '';
    outer.begin(wasm);
    wasm.setShapeProperties(0, shapes[0].ppi, shapes[0].ci, { horzOffset: shapes[0].origHorzOffset + 750 });
    const l1 = stateLabelOf(wasm) ?? '';
    inner.begin(wasm);
    wasm.setShapeProperties(0, shapes[1].ppi, shapes[1].ci, { horzOffset: shapes[1].origHorzOffset + 750 });
    const l2 = stateLabelOf(wasm) ?? '';
    const innerCommand = inner.finish(wasm);
    const outerCommand = outer.finish(wasm);
    assert.ok(innerCommand && outerCommand);
    assert.deepEqual(innerCommand.recordedLabels?.(wasm), { undoLabel: l2, redoLabel: l1 }, 'the inner gesture keeps its own pair');
    assert.deepEqual(outerCommand.recordedLabels?.(wasm), { undoLabel: l2, redoLabel: l0 }, 'the outer gesture keeps its own pair');
    innerCommand.discard?.(wasm);
    outerCommand.discard?.(wasm);

    const a = wasm.saveSnapshot();
    wasm.setShapeProperties(0, shapes[0].ppi, shapes[0].ci, { horzOffset: shapes[0].origHorzOffset });
    const b = wasm.saveSnapshot();
    const forged = SnapshotCommand.recorded('moveObjects', a, b, position(0, 0), position(0, 0), l0, stateLabelOf(wasm));
    assert.equal(forged.recordedLabels(wasm), null, 'explicit labels that disagree with the snapshots are refused');
    history.recordWithoutExecute(forged, wasm);
    assert.equal(history.canUndo(), false, 'a forged pair is not recorded');
    assert.equal(rawSnapshotLabels(wasm, a), undefined, 'the refused pair released its snapshots');
    assert.equal(rawSnapshotLabels(wasm, b), undefined);
  } finally { history.clear(wasm); wasm.dispose(); }
});

test('C9 stale gesture finish and cancellation preserve the new document snapshot ids and undo bytes', () => {
  for (const cancel of [false, true]) {
    const wasm = new WasmBridge(), history = new CommandHistory();
    try {
      wasm.loadDocument(r8Field(), 'A.hwpx');
      const recorder = new ObjectGestureRecorder('moveObjects', position(0, 0));
      recorder.begin(wasm);
      wasm.loadDocument(fixture(para('<hp:t>NEW</hp:t>')), 'B.hwpx');
      const original = wasm.exportHwpx();
      history.execute(new ApplyCharFormatCommand(position(0, 0), position(0, 1), { bold: true }), wasm);
      if (cancel) recorder.cancel(wasm);
      else {
        const command = recorder.finish(wasm);
        assert.equal(command, null, 'a stale finish returns null without allocating or recording in B');
        history.recordWithoutExecute(command, wasm);
      }
      assert.ok(rawSnapshotLabels(wasm, 0));
      assert.ok(rawSnapshotLabels(wasm, 1));
      assert.equal(rawSnapshotLabels(wasm, 2), undefined, 'a stale finish must not allocate in B');
      assert.equal(history.entryLabels().undo.length, 1);
      assert.ok(history.undo(wasm));
      assert.deepEqual(wasm.exportHwpx(), original);
    } finally { history.clear(wasm); wasm.dispose(); }
  }
});

test('C9 pointer interruption clears every latch and resources even when finish fails', () => {
  const wasm = new WasmBridge(), history = new CommandHistory();
  const previousDocument = globalThis.document;
  const documentEvents = new EventTarget();
  Object.assign(globalThis, { document: documentEvents });
  try {
    wasm.loadDocument(r8Field(), 'pointer-fault.hwpx');
    const recorder = new ObjectGestureRecorder('moveObjects', position(0, 0));
    recorder.begin(wasm);
    const terminal = () => { throw new Error('injected finish'); };
    const host = {
      wasm, history, container: { style: { cursor: 'move' } },
      isMoveDragging: false, moveDragState: null,
      isPictureMoveDragging: true, pictureMoveState: { recorder },
      isPictureResizeDragging: true, pictureResizeState: {},
      isPictureRotateDragging: true, pictureRotateState: {},
      isLineEndpointDragging: true, lineEndpointState: {},
      isResizeDragging: true, resizeDragState: {}, isCellDragSelecting: true, isDragging: true,
      cellDragAnchorRange: {}, onMouseUpBound() {}, finishPictureMoveDrag: terminal,
    };
    assert.throws(() => InputHandler.prototype.settleOpenPointerGesture.call(host, 'interrupt'), /injected finish/);
    assert.equal(hasOpenPointerGesture(host), false);
    assert.equal(host.container.style.cursor, '');
    assert.equal(rawSnapshotLabels(wasm, 0), undefined);
    assert.equal(recorder.started, false);
  } finally { Object.assign(globalThis, { document: previousDocument }); history.clear(wasm); wasm.dispose(); }
});

test('C9 picture move cleanup releases both captures when history registration throws', () => {
  const wasm = new WasmBridge(), history = new CommandHistory();
  try {
    wasm.loadDocument(r8Field(), 'picture-fault.hwpx');
    const recorder = new ObjectGestureRecorder('moveObjects', position(0, 0));
    recorder.begin(wasm);
    wasm.insertText(0, 0, 0, 'X');
    const host = {
      wasm, history: { recordWithoutExecute() { throw new Error('injected registration'); } },
      isPictureMoveDragging: true, pictureMoveState: { recorder }, dragRafId: 0,
      container: { style: { cursor: 'move' } },
    };
    assert.throws(() => finishPictureMoveDrag.call(host), /injected registration/);
    assert.equal(hasOpenPointerGesture(host), false);
    assert.equal(rawSnapshotLabels(wasm, 0), undefined);
    assert.equal(rawSnapshotLabels(wasm, 1), undefined);
  } finally { history.clear(wasm); wasm.dispose(); }
});

// ─── C10: R10-B & R10-C regressions (press-epoch replacement cancellation + full gesture coverage) ───

const c10Png = Uint8Array.from(
  atob('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jqK8AAAAASUVORK5CYII='),
  ch => ch.charCodeAt(0),
);

const twoColTable = (left: string, right: string): string => para(
  '<hp:tbl rowCnt="1" colCnt="2" borderFillIDRef="1"><hp:tr>'
  + `<hp:tc borderFillIDRef="1"><hp:subList>${left}</hp:subList><hp:cellAddr colAddr="0" rowAddr="0"/><hp:cellSpan colSpan="1" rowSpan="1"/><hp:cellSz width="10000" height="1500"/></hp:tc>`
  + `<hp:tc borderFillIDRef="1"><hp:subList>${right}</hp:subList><hp:cellAddr colAddr="1" rowAddr="0"/><hp:cellSpan colSpan="1" rowSpan="1"/><hp:cellSz width="10000" height="1500"/></hp:tc>`
  + '</hp:tr></hp:tbl>',
);

function withPointerHarness<T>(fn: (ctx: { docEvents: EventTarget; flush: () => void }) => T): T {
  const docEvents = new EventTarget();
  const rafs: Array<(time: number) => void> = [];
  const savedDoc = Object.getOwnPropertyDescriptor(globalThis, 'document');
  const savedRaf = globalThis.requestAnimationFrame;
  const savedCaf = globalThis.cancelAnimationFrame;
  Object.defineProperty(globalThis, 'document', { configurable: true, writable: true, value: docEvents });
  globalThis.requestAnimationFrame = cb => { rafs.push(cb); return rafs.length; };
  // Intentionally a no-op like R10 probes to prove queued RAFs also self-reject stale epochs/states.
  globalThis.cancelAnimationFrame = () => {};
  try {
    return fn({
      docEvents,
      flush: () => { while (rafs.length > 0) rafs.shift()!(0); },
    });
  } finally {
    if (savedDoc) Object.defineProperty(globalThis, 'document', savedDoc);
    else Reflect.deleteProperty(globalThis, 'document');
    globalThis.requestAnimationFrame = savedRaf;
    globalThis.cancelAnimationFrame = savedCaf;
  }
}

test('C10 R10-B loadDocument and createNewDocument synchronously cancel every open pointer gesture before deactivate', () => {
  const gestureKinds = [
    'table-move',
    'picture-move-unbegun',
    'picture-move-begun',
    'picture-rotate',
    'picture-resize',
    'line-endpoint',
    'table-border-resize',
  ] as const;

  for (const replacement of ['loadDocument', 'createNewDocument'] as const) {
    for (const kind of gestureKinds) {
      withPointerHarness(({ docEvents, flush }) => {
        const wasm = new WasmBridge(), history = new CommandHistory();
        try {
          const docABytes = fixture(twoColTable(para('<hp:t>L</hp:t>'), para('<hp:t>R</hp:t>')) + para('<hp:t>AB</hp:t>'));
          const docBBytes = fixture(twoColTable(para('<hp:t>NL</hp:t>'), para('<hp:t>NR</hp:t>')) + para('<hp:t>CD</hp:t>'));
          wasm.loadDocument(docABytes, 'A.hwpx');
          const picA = wasm.insertPicture(0, 1, 0, c10Png, 3000, 3000, 1, 1, 'png');
          wasm.setPictureProperties(0, picA.paraIdx, picA.controlIdx, { treatAsChar: false, horzOffset: 750, vertOffset: 750 });
          const lineA = wasm.createShapeControl({
            sectionIdx: 0, paraIdx: 1, charOffset: 0, width: 3000, height: 1500,
            horzOffset: 1500, vertOffset: 1500, shapeType: 'line', treatAsChar: false,
          });

          let selectedTable = { sec: 0, ppi: 0, ci: 0 };
          const sc = { clientWidth: 1000, getBoundingClientRect: () => ({ left: 0, top: 0 }) };
          const host: any = {
            active: true, wasm, history, dragRafId: 0,
            cursor: {
              isInTableObjectSelection: () => kind === 'table-move',
              getSelectedTableRef: () => ({ ...selectedTable }),
              updateSelectedTableRef: (sec: number, ppi: number, ci: number) => { selectedTable = { sec, ppi, ci }; },
              isInCellSelectionMode: () => false,
            },
            container: {
              style: { cursor: '' }, clientWidth: 1000, clientHeight: 1000,
              getBoundingClientRect: () => ({ left: 0, top: 0 }), querySelector: () => sc,
            },
            getTableObjectBBox: () => ({ x: 0, y: 0, width: 200, height: 100 }),
            tableResizeRenderer: {
              computeBorderLines: (boxes: any[]) => TableResizeRenderer.prototype.computeBorderLines(boxes),
              showDragMarker() {},
              clear() {},
            },
            pictureObjectRenderer: { renderDragPreview() {}, clearDragPreview() {}, clear() {} },
            viewportManager: { getZoom: () => 1 },
            virtualScroll: { getPageAtXY: () => 0, getPageOffset: () => 0, getPageWidth: () => 800, getPageLeft: () => 0 },
            textarea: { focus() {} },
            eventBus: { emit() {} },
            renderTableObjectSelection() {},
            renderPictureObjectSelection() {},
            executeOperation: InputHandler.prototype.executeOperation,
          };
          host.onMouseUpBound = (event: MouseEvent) => onMouseUp.call(host, event);
          host.getObjectProperties = (r: any) => getObjectProperties.call(host, r);
          host.findPictureBbox = (r: any) => findPictureBbox.call(host, r);
          host.updateMoveDrag = (e: MouseEvent) => updateMoveDrag.call(host, e);
          host.finishMoveDrag = () => finishMoveDrag.call(host);
          host.updatePictureMoveDrag = (e: MouseEvent) => updatePictureMoveDrag.call(host, e);
          host.finishPictureMoveDrag = () => finishPictureMoveDrag.call(host);
          host.updatePictureRotateDrag = (e: MouseEvent) => updatePictureRotateDrag.call(host, e);
          host.finishPictureRotateDrag = (e?: MouseEvent) => finishPictureRotateDrag.call(host, e);
          host.updatePictureResizeDrag = (e: MouseEvent) => updatePictureResizeDrag.call(host, e);
          host.finishPictureResizeDrag = (e?: MouseEvent) => finishPictureResizeDrag.call(host, e);
          host.updateLineEndpointDrag = (e: MouseEvent) => updateLineEndpointDrag.call(host, e);
          host.finishLineEndpointDrag = () => finishLineEndpointDrag.call(host);
          host.updateResizeDrag = (e: MouseEvent) => updateResizeDrag.call(host, e);
          host.finishResizeDrag = (e?: MouseEvent) => finishResizeDrag.call(host, e);
          host.cleanupResizeDrag = () => cleanupResizeDrag.call(host);
          host.cleanupPictureResizeDrag = () => cleanupPictureResizeDrag.call(host);

          const picRef = { sec: 0, ppi: picA.paraIdx, ci: picA.controlIdx, type: 'image' as const };
          const lineRef = { sec: 0, ppi: lineA.paraIdx, ci: lineA.controlIdx, type: 'line' as const };

          if (kind === 'table-move') {
            onClick.call(host, { button: 0, detail: 1, clientX: 10, clientY: 10, target: { closest: () => null }, preventDefault() {} } as unknown as MouseEvent);
            onMouseMove.call(host, { clientX: 25, clientY: 10, buttons: 1 } as MouseEvent);
            flush();
          } else if (kind === 'picture-move-unbegun' || kind === 'picture-move-begun') {
            const pa = host.getObjectProperties(picRef);
            host.isPictureMoveDragging = true;
            host.pictureMoveState = {
              ref: { ...picRef, origHorzOffset: pa.horzOffset, origVertOffset: pa.vertOffset },
              origHorzOffset: pa.horzOffset, origVertOffset: pa.vertOffset,
              startPageX: 10, startPageY: 10, lastPageX: 10, lastPageY: 10, totalDeltaH: 0, totalDeltaV: 0,
              pageIndex: 0, beforeLabel: stateLabelOf(wasm),
              multiRefs: [{ ...picRef, origHorzOffset: pa.horzOffset, origVertOffset: pa.vertOffset }],
            };
            docEvents.addEventListener('mouseup', host.onMouseUpBound, { once: true });
            if (kind === 'picture-move-begun') {
              onMouseMove.call(host, { clientX: 25, clientY: 10, buttons: 1 } as MouseEvent);
              flush();
            }
          } else if (kind === 'picture-rotate') {
            host.getObjectProperties(picRef);
            host.isPictureRotateDragging = true;
            host.pictureRotateState = {
              ref: picRef, origAngle: 0, centerX: 50, centerY: 50, startAngle: 0,
              pageIndex: 0, epoch: documentEpochOf(wasm), beforeLabel: stateLabelOf(wasm),
            };
            docEvents.addEventListener('mouseup', host.onMouseUpBound, { once: true });
            // Queue a pre-reload RAF without flushing yet to test orphaned RAF cancellation too
            onMouseMove.call(host, { clientX: 50, clientY: 100, buttons: 1, ctrlKey: false } as MouseEvent);
          } else if (kind === 'picture-resize') {
            const pa = host.getObjectProperties(picRef);
            const bb = host.findPictureBbox(picRef)!;
            host.isPictureResizeDragging = true;
            host.pictureResizeState = {
              dir: 'se', ref: picRef, origWidth: pa.width, origHeight: pa.height,
              rotationAngle: 0, startClientX: 20, startClientY: 20, lastClientX: 20, lastClientY: 20,
              pageIndex: bb.pageIndex, bbox: { x: bb.x, y: bb.y, w: bb.w, h: bb.h },
              epoch: documentEpochOf(wasm), beforeLabel: stateLabelOf(wasm),
            };
            docEvents.addEventListener('mouseup', host.onMouseUpBound, { once: true });
          } else if (kind === 'line-endpoint') {
            host.getObjectProperties(lineRef);
            const bb = host.findPictureBbox(lineRef)!;
            host.isLineEndpointDragging = true;
            host.lineEndpointState = {
              ref: lineRef, endpoint: 'end', pageIndex: bb.pageIndex,
              pageLeft: 0, pageOffset: 0, zoom: 1,
              epoch: documentEpochOf(wasm), beforeLabel: stateLabelOf(wasm),
            };
            docEvents.addEventListener('mouseup', host.onMouseUpBound, { once: true });
          } else if (kind === 'table-border-resize') {
            const allBboxes = wasm.getTableCellBboxes(0, 0, 0);
            const pageBboxes = allBboxes.filter(b => b.pageIndex === 0);
            const { colLines } = host.tableResizeRenderer.computeBorderLines(pageBboxes);
            host.cachedTableRef = { sec: 0, ppi: 0, ci: 0 };
            host.cachedCellBboxes = allBboxes;
            startResizeDrag.call(host, { type: 'col', index: colLines[1].index, pageIndex: 0 }, colLines[1].x, 10, pageBboxes);
            onMouseMove.call(host, { clientX: colLines[1].x + 20, clientY: 10, buttons: 1 } as MouseEvent);
          }

          assert.equal(hasOpenPointerGesture(host), true, `${replacement}/${kind}: gesture opened in A`);

          if (replacement === 'loadDocument') {
            wasm.loadDocument(docBBytes, 'B.hwpx');
            const picB = wasm.insertPicture(0, 1, 0, c10Png, 3000, 3000, 1, 1, 'png');
            wasm.setPictureProperties(0, picB.paraIdx, picB.controlIdx, { treatAsChar: false, horzOffset: 750, vertOffset: 750 });
          } else {
            wasm.createNewDocument();
          }
          history.clear(wasm);

          const bSnap = wasm.exportHwpx();
          const bLabel = stateLabelOf(wasm);

          assert.equal(hasOpenPointerGesture(host), false, `${replacement}/${kind}: synchronously cancelled before deactivate`);
          flush();
          onMouseMove.call(host, { clientX: 80, clientY: 80, buttons: 1, ctrlKey: false } as MouseEvent);
          flush();
          docEvents.dispatchEvent(new Event('mouseup'));
          InputHandler.prototype.settleOpenPointerGesture.call(host, 'interrupt');

          assert.deepEqual(wasm.exportHwpx(), bSnap, `${replacement}/${kind}: replacement document bytes mutated`);
          assert.equal(stateLabelOf(wasm), bLabel, `${replacement}/${kind}: replacement document label changed`);
          assert.deepEqual(history.entryLabels().undo, [], `${replacement}/${kind}: stale gesture recorded into replacement history`);
        } finally {
          history.clear(wasm);
          wasm.dispose();
        }
      });
    }
  }
});

function fullSemanticState(wasm: WasmBridge, canExportHwpx: boolean): { snapshot: string; hwpx: Uint8Array | null } {
  const paragraphs: unknown[] = [];
  for (let p = 0; p < wasm.getParagraphCount(0); p++) {
    const controls: unknown[] = [];
    for (let ci = 0; ci < 6; ci++) {
      try { controls.push({ kind: 'shape', ci, props: wasm.getShapeProperties(0, p, ci) }); } catch { /* not shape */ }
      try { controls.push({ kind: 'pic', ci, props: wasm.getPictureProperties(0, p, ci) }); } catch { /* not pic */ }
      try {
        controls.push({
          kind: 'tbl',
          ci,
          props: wasm.getTableProperties(0, p, ci),
          cells: wasm.getTableCellBboxes(0, p, ci),
        });
      } catch { /* not tbl */ }
    }
    paragraphs.push({ text: wasm.getTextRange(0, p, 0, wasm.getParagraphLength(0, p)), controls });
  }
  return {
    snapshot: JSON.stringify({
      paragraphs,
      layout: wasm.getPageControlLayout(0),
      svg: wasm.renderPageSvg(0),
    }),
    hwpx: canExportHwpx ? wasm.exportHwpx() : null,
  };
}

test('C10 R10-C every pointer gesture kind records one exact history entry on mouseup and interrupt and preserves older typing', () => {
  const terminals = ['mouseup', 'buttons0', 'Escape', 'blur'] as const;
  const kinds = [
    'picture-move',
    'picture-resize-single',
    'picture-resize-multi',
    'picture-resize-group',
    'picture-rotate',
    'line-endpoint',
    'table-move',
    'table-move-back-to-origin',
    'table-border-resize',
  ] as const;

  for (const kind of kinds) {
    for (const terminal of terminals) {
      withPointerHarness(({ docEvents, flush }) => {
        const wasm = new WasmBridge(), history = new CommandHistory();
        try {
          wasm.loadDocument(
            fixture(twoColTable(para('<hp:t>L</hp:t>'), para('<hp:t>R</hp:t>')) + para('<hp:t>ONE</hp:t>') + para('<hp:t>AB</hp:t>')),
            `${kind}-${terminal}.hwpx`,
          );
          const usesRuntimeShapes = kind === 'picture-resize-multi' || kind === 'picture-resize-group' || kind === 'line-endpoint';
          const canExportHwpx = !usesRuntimeShapes;

          const pic = wasm.insertPicture(0, 1, 0, c10Png, 3000, 3000, 1, 1, 'png');
          wasm.setPictureProperties(0, pic.paraIdx, pic.controlIdx, { treatAsChar: false, horzOffset: 1500, vertOffset: 1500 });

          let s1: { paraIdx: number; controlIdx: number } | null = null;
          let s2: { paraIdx: number; controlIdx: number } | null = null;
          let line: { paraIdx: number; controlIdx: number } | null = null;
          let groupRef: { sec: number; ppi: number; ci: number; type: 'group' } | null = null;

          if (kind === 'picture-resize-multi') {
            s1 = wasm.createShapeControl({
              sectionIdx: 0, paraIdx: 1, charOffset: 0, width: 3000, height: 3000,
              horzOffset: 1500, vertOffset: 1500, shapeType: 'rectangle', treatAsChar: false,
            });
            s2 = wasm.createShapeControl({
              sectionIdx: 0, paraIdx: 1, charOffset: 0, width: 3000, height: 3000,
              horzOffset: 6000, vertOffset: 1500, shapeType: 'rectangle', treatAsChar: false,
            });
          } else if (kind === 'picture-resize-group') {
            const g1 = wasm.createShapeControl({
              sectionIdx: 0, paraIdx: 1, charOffset: 0, width: 2250, height: 2250,
              horzOffset: 9000, vertOffset: 1500, shapeType: 'rectangle', treatAsChar: false,
            });
            const g2 = wasm.createShapeControl({
              sectionIdx: 0, paraIdx: 1, charOffset: 0, width: 2250, height: 2250,
              horzOffset: 12000, vertOffset: 1500, shapeType: 'rectangle', treatAsChar: false,
            });
            const grouped = wasm.groupShapes(0, [
              { paraIdx: g1.paraIdx, controlIdx: g1.controlIdx },
              { paraIdx: g2.paraIdx, controlIdx: g2.controlIdx },
            ]);
            assert.equal(grouped.ok, true);
            groupRef = { sec: 0, ppi: grouped.paraIdx, ci: grouped.controlIdx, type: 'group' };
          } else if (kind === 'line-endpoint') {
            line = wasm.createShapeControl({
              sectionIdx: 0, paraIdx: 1, charOffset: 0, width: 4500, height: 1500,
              horzOffset: 1500, vertOffset: 6000, shapeType: 'line', treatAsChar: false,
            });
          }
          history.clear(wasm);

          // Two distinct older typing entries (non-merged timestamps)
          const targetPara = wasm.getParagraphCount(0) - 1;
          const initial = fullSemanticState(wasm, canExportHwpx);
          history.execute(new InsertTextCommand(position(targetPara, 2), 'X', 1000), wasm);
          history.execute(new InsertTextCommand(position(targetPara, 3), 'Y', 5000), wasm);
          assert.equal(history.entryLabels().undo.length, 2);
          const preGesture = fullSemanticState(wasm, canExportHwpx);
          const preGestureLabel = stateLabelOf(wasm)!;

          let selectedTable = { sec: 0, ppi: 0, ci: 0 };
          const sc = { clientWidth: 1000, getBoundingClientRect: () => ({ left: 0, top: 0 }) };
          const host: any = {
            active: true, wasm, history, dragRafId: 0,
            cursor: {
              isInTableObjectSelection: () => kind.startsWith('table-move'),
              getSelectedTableRef: () => ({ ...selectedTable }),
              updateSelectedTableRef: (sec: number, ppi: number, ci: number) => { selectedTable = { sec, ppi, ci }; },
              isInCellSelectionMode: () => false,
            },
            container: {
              style: { cursor: '' }, clientWidth: 1000, clientHeight: 1000,
              getBoundingClientRect: () => ({ left: 0, top: 0 }), querySelector: () => sc,
            },
            getTableObjectBBox: () => ({ x: 0, y: 0, width: 200, height: 100 }),
            tableResizeRenderer: {
              computeBorderLines: (boxes: any[]) => TableResizeRenderer.prototype.computeBorderLines(boxes),
              showDragMarker() {},
              clear() {},
            },
            pictureObjectRenderer: { renderDragPreview() {}, clearDragPreview() {}, clear() {} },
            viewportManager: { getZoom: () => 1 },
            virtualScroll: { getPageAtXY: () => 0, getPageOffset: () => 0, getPageWidth: () => 800, getPageLeft: () => 0 },
            textarea: { focus() {} },
            eventBus: { emit() {} },
            renderTableObjectSelection() {},
            renderPictureObjectSelection() {},
            executeOperation: InputHandler.prototype.executeOperation,
          };
          host.onMouseUpBound = (event: MouseEvent) => onMouseUp.call(host, event);
          host.getObjectProperties = (r: any) => getObjectProperties.call(host, r);
          host.findPictureBbox = (r: any) => findPictureBbox.call(host, r);
          host.updateMoveDrag = (e: MouseEvent) => updateMoveDrag.call(host, e);
          host.finishMoveDrag = () => finishMoveDrag.call(host);
          host.updatePictureMoveDrag = (e: MouseEvent) => updatePictureMoveDrag.call(host, e);
          host.finishPictureMoveDrag = () => finishPictureMoveDrag.call(host);
          host.updatePictureRotateDrag = (e: MouseEvent) => updatePictureRotateDrag.call(host, e);
          host.finishPictureRotateDrag = (e?: MouseEvent) => finishPictureRotateDrag.call(host, e);
          host.updatePictureResizeDrag = (e: MouseEvent) => updatePictureResizeDrag.call(host, e);
          host.finishPictureResizeDrag = (e?: MouseEvent) => finishPictureResizeDrag.call(host, e);
          host.updateLineEndpointDrag = (e: MouseEvent) => updateLineEndpointDrag.call(host, e);
          host.finishLineEndpointDrag = () => finishLineEndpointDrag.call(host);
          host.updateResizeDrag = (e: MouseEvent) => updateResizeDrag.call(host, e);
          host.finishResizeDrag = (e?: MouseEvent) => finishResizeDrag.call(host, e);
          host.cleanupResizeDrag = () => cleanupResizeDrag.call(host);
          host.cleanupPictureResizeDrag = () => cleanupPictureResizeDrag.call(host);

          const picRef = { sec: 0, ppi: pic.paraIdx, ci: pic.controlIdx, type: 'image' as const };
          let endClientX = 60;
          let endClientY = 60;
          let expectedType = '';

          if (kind === 'picture-move') {
            expectedType = 'snapshot:moveObjects';
            const p1 = host.getObjectProperties(picRef);
            host.isPictureMoveDragging = true;
            host.pictureMoveState = {
              ref: picRef, origHorzOffset: p1.horzOffset, origVertOffset: p1.vertOffset,
              startPageX: 20, startPageY: 20, lastPageX: 20, lastPageY: 20, totalDeltaH: 0, totalDeltaV: 0,
              pageIndex: 0, epoch: documentEpochOf(wasm), beforeLabel: preGestureLabel,
            };
            docEvents.addEventListener('mouseup', host.onMouseUpBound, { once: true });
            onMouseMove.call(host, { clientX: 50, clientY: 35, buttons: 1 } as MouseEvent);
            flush();
            endClientX = 50; endClientY = 35;
          } else if (kind === 'picture-resize-single') {
            expectedType = 'snapshot:resizeObjects';
            const p1 = host.getObjectProperties(picRef);
            const bb1 = host.findPictureBbox(picRef)!;
            host.isPictureResizeDragging = true;
            host.pictureResizeState = {
              dir: 'se', ref: picRef, origWidth: p1.width, origHeight: p1.height, rotationAngle: 0,
              startClientX: 40, startClientY: 40, lastClientX: 40, lastClientY: 40,
              pageIndex: bb1.pageIndex, bbox: { x: bb1.x, y: bb1.y, w: bb1.w, h: bb1.h },
              epoch: documentEpochOf(wasm), beforeLabel: preGestureLabel,
            };
            docEvents.addEventListener('mouseup', host.onMouseUpBound, { once: true });
            onMouseMove.call(host, { clientX: 70, clientY: 65, buttons: 1 } as MouseEvent);
            flush();
            endClientX = 70; endClientY = 65;
          } else if (kind === 'picture-resize-multi') {
            expectedType = 'snapshot:resizeObjects';
            const ref1 = { sec: 0, ppi: s1!.paraIdx, ci: s1!.controlIdx, type: 'shape' as const };
            const ref2 = { sec: 0, ppi: s2!.paraIdx, ci: s2!.controlIdx, type: 'shape' as const };
            const p1 = host.getObjectProperties(ref1);
            const p2 = host.getObjectProperties(ref2);
            const bb1 = host.findPictureBbox(ref1)!;
            const bb2 = host.findPictureBbox(ref2)!;
            const minX = Math.min(bb1.x, bb2.x), minY = Math.min(bb1.y, bb2.y);
            const maxX = Math.max(bb1.x + bb1.w, bb2.x + bb2.w), maxY = Math.max(bb1.y + bb1.h, bb2.y + bb2.h);
            host.isPictureResizeDragging = true;
            host.pictureResizeState = {
              dir: 'se', ref: ref1,
              origWidth: Math.round((maxX - minX) * 75), origHeight: Math.round((maxY - minY) * 75),
              startClientX: 100, startClientY: 50, lastClientX: 100, lastClientY: 50,
              pageIndex: bb1.pageIndex, bbox: { x: minX, y: minY, w: maxX - minX, h: maxY - minY },
              multiRefs: [
                { ...ref1, origWidth: p1.width, origHeight: p1.height, origHorzOffset: p1.horzOffset, origVertOffset: p1.vertOffset, bboxX: bb1.x, bboxY: bb1.y },
                { ...ref2, origWidth: p2.width, origHeight: p2.height, origHorzOffset: p2.horzOffset, origVertOffset: p2.vertOffset, bboxX: bb2.x, bboxY: bb2.y },
              ],
              epoch: documentEpochOf(wasm), beforeLabel: preGestureLabel,
            };
            docEvents.addEventListener('mouseup', host.onMouseUpBound, { once: true });
            onMouseMove.call(host, { clientX: 140, clientY: 80, buttons: 1 } as MouseEvent);
            flush();
            endClientX = 140; endClientY = 80;
          } else if (kind === 'picture-resize-group') {
            expectedType = 'snapshot:resizeObjects';
            const pg = host.getObjectProperties(groupRef!);
            const bbg = host.findPictureBbox(groupRef!)!;
            host.isPictureResizeDragging = true;
            host.pictureResizeState = {
              dir: 'se', ref: groupRef!, origWidth: pg.width, origHeight: pg.height, rotationAngle: 0,
              startClientX: 120, startClientY: 50, lastClientX: 120, lastClientY: 50,
              pageIndex: bbg.pageIndex, bbox: { x: bbg.x, y: bbg.y, w: bbg.w, h: bbg.h },
              epoch: documentEpochOf(wasm), beforeLabel: preGestureLabel,
            };
            docEvents.addEventListener('mouseup', host.onMouseUpBound, { once: true });
            onMouseMove.call(host, { clientX: 155, clientY: 75, buttons: 1 } as MouseEvent);
            flush();
            endClientX = 155; endClientY = 75;
          } else if (kind === 'picture-rotate') {
            expectedType = 'snapshot:rotateObject';
            host.isPictureRotateDragging = true;
            host.pictureRotateState = {
              ref: picRef, origAngle: 0, centerX: 50, centerY: 50, startAngle: Math.atan2(0, 50),
              pageIndex: 0, epoch: documentEpochOf(wasm), beforeLabel: preGestureLabel,
            };
            docEvents.addEventListener('mouseup', host.onMouseUpBound, { once: true });
            onMouseMove.call(host, { clientX: 50, clientY: 100, buttons: 1, ctrlKey: false } as MouseEvent);
            flush();
            endClientX = 50; endClientY = 100;
          } else if (kind === 'line-endpoint') {
            expectedType = 'snapshot:moveLineEndpoint';
            const lineRef = { sec: 0, ppi: line!.paraIdx, ci: line!.controlIdx, type: 'line' as const };
            const bbl = host.findPictureBbox(lineRef)!;
            host.isLineEndpointDragging = true;
            host.lineEndpointState = {
              ref: lineRef, endpoint: 'end', pageIndex: bbl.pageIndex,
              pageLeft: 0, pageOffset: 0, zoom: 1,
              epoch: documentEpochOf(wasm), beforeLabel: preGestureLabel,
            };
            docEvents.addEventListener('mouseup', host.onMouseUpBound, { once: true });
            onMouseMove.call(host, { clientX: 120, clientY: 130, buttons: 1, shiftKey: false } as MouseEvent);
            flush();
            endClientX = 120; endClientY = 130;
          } else if (kind === 'table-move' || kind === 'table-move-back-to-origin') {
            expectedType = 'moveTable';
            onClick.call(host, { button: 0, detail: 1, clientX: 10, clientY: 10, target: { closest: () => null }, preventDefault() {} } as unknown as MouseEvent);
            onMouseMove.call(host, { clientX: 30, clientY: 10, buttons: 1 } as MouseEvent);
            flush();
            if (kind === 'table-move-back-to-origin') {
              onMouseMove.call(host, { clientX: 10, clientY: 10, buttons: 1 } as MouseEvent);
              flush();
              endClientX = 10; endClientY = 10;
            } else {
              endClientX = 30; endClientY = 10;
            }
          } else if (kind === 'table-border-resize') {
            expectedType = 'snapshot:resizeTableCells';
            const allBboxes = wasm.getTableCellBboxes(0, 0, 0);
            const pageBboxes = allBboxes.filter(b => b.pageIndex === 0);
            const { colLines } = host.tableResizeRenderer.computeBorderLines(pageBboxes);
            host.cachedTableRef = { sec: 0, ppi: 0, ci: 0 };
            host.cachedCellBboxes = allBboxes;
            startResizeDrag.call(host, { type: 'col', index: colLines[1].index, pageIndex: 0 }, colLines[1].x, 10, pageBboxes);
            endClientX = colLines[1].x + 25;
            endClientY = 10;
            onMouseMove.call(host, { clientX: endClientX, clientY: endClientY, buttons: 1 } as MouseEvent);
            flush();
          }

          if (terminal === 'mouseup') {
            onMouseUp.call(host, { clientX: endClientX, clientY: endClientY, buttons: 0 } as MouseEvent);
          } else if (terminal === 'buttons0') {
            onMouseMove.call(host, { clientX: endClientX, clientY: endClientY, buttons: 0 } as MouseEvent);
          } else if (terminal === 'Escape') {
            const k = keyEvent('Escape');
            onKeyDown.call(host, k.event);
            assert.equal(k.prevented(), 1);
          } else if (terminal === 'blur') {
            InputHandler.prototype.settleOpenPointerGesture.call(host, 'interrupt');
          }

          const tag = `${kind}/${terminal}`;
          assert.equal(hasOpenPointerGesture(host), false, `${tag}: latch must close`);
          const postGesture = fullSemanticState(wasm, canExportHwpx);
          const postGestureLabel = stateLabelOf(wasm)!;
          if (kind !== 'table-move-back-to-origin') {
            assert.notEqual(postGesture.snapshot, preGesture.snapshot, `${tag}: gesture must mutate state`);
          }

          const entries = history.entryLabels().undo;
          assert.equal(entries.length, 3, `${tag}: gesture must add exactly 1 history entry`);
          assert.equal(entries[2].type, expectedType, `${tag}: entry type`);
          assert.equal(entries[2].redoLabel, preGestureLabel, `${tag}: redoLabel matches pre-gesture label`);
          assert.equal(entries[2].undoLabel, postGestureLabel, `${tag}: undoLabel matches post-gesture label`);

          // Undo gesture -> exact pre-gesture state & bytes, older typing still intact!
          assert.notEqual(history.undo(wasm), null, `${tag}: gesture undo must succeed`);
          assert.equal(stateLabelOf(wasm), preGestureLabel, `${tag}: undo restores pre-gesture label`);
          const afterGestureUndo = fullSemanticState(wasm, canExportHwpx);
          assert.equal(afterGestureUndo.snapshot, preGesture.snapshot, `${tag}: undo restores pre-gesture semantic state`);
          if (canExportHwpx) assert.deepEqual(afterGestureUndo.hwpx, preGesture.hwpx, `${tag}: undo restores pre-gesture bytes`);
          assert.equal(history.entryLabels().undo.length, 2, `${tag}: older typing entries preserved`);
          assert.equal(history.canUndo(), true, `${tag}: older typing stays undoable`);

          // Undo both older typing edits!
          assert.notEqual(history.undo(wasm), null, `${tag}: undo Y`);
          assert.equal(wasm.getTextRange(0, targetPara, 0, 3), 'ABX', `${tag}: text after undoing Y`);
          assert.notEqual(history.undo(wasm), null, `${tag}: undo X`);
          assert.equal(wasm.getTextRange(0, targetPara, 0, 2), 'AB', `${tag}: text after undoing X`);
          const afterFullUndo = fullSemanticState(wasm, canExportHwpx);
          assert.equal(afterFullUndo.snapshot, initial.snapshot, `${tag}: full undo restores initial state`);
          if (canExportHwpx) assert.deepEqual(afterFullUndo.hwpx, initial.hwpx, `${tag}: full undo restores initial bytes`);
          assert.equal(history.canUndo(), false, `${tag}: stack exhausted cleanly`);

          // Redo all three!
          assert.notEqual(history.redo(wasm), null, `${tag}: redo X`);
          assert.notEqual(history.redo(wasm), null, `${tag}: redo Y`);
          assert.notEqual(history.redo(wasm), null, `${tag}: redo gesture`);
          assert.equal(stateLabelOf(wasm), postGestureLabel, `${tag}: full redo restores post-gesture label`);
          const afterFullRedo = fullSemanticState(wasm, canExportHwpx);
          assert.equal(afterFullRedo.snapshot, postGesture.snapshot, `${tag}: full redo restores post-gesture state`);
          if (canExportHwpx) assert.deepEqual(afterFullRedo.hwpx, postGesture.hwpx, `${tag}: full redo restores post-gesture bytes`);
        } finally {
          history.clear(wasm);
          wasm.dispose();
        }
      });
    }
  }
});

test('C10 keyboard table cell resize and zero-motion pointer gestures keep history exact without leaking snapshots', () => {
  const wasm = new WasmBridge(), history = new CommandHistory();
  try {
    wasm.loadDocument(
      fixture(twoColTable(para('<hp:t>L</hp:t>'), para('<hp:t>R</hp:t>')) + para('<hp:t>AB</hp:t>')),
      'c10-kbd-table.hwpx',
    );
    history.execute(new InsertTextCommand(position(1, 2), 'Z', 1000), wasm);
    const typedBytes = wasm.exportHwpx();
    const typedLabel = stateLabelOf(wasm)!;

    const host: any = {
      active: true, wasm, history,
      cursor: {
        getCellTableContext: () => ({ sec: 0, ppi: 0, ci: 0 }),
        getSelectedCellRange: () => ({ startRow: 0, startCol: 0, endRow: 0, endCol: 0 }),
      },
      eventBus: { emit() {} },
      updateCellSelection() {},
    };
    resizeCellByKeyboard.call(host, 'ArrowRight');
    const afterCellResizeBytes = wasm.exportHwpx();
    assert.notDeepEqual(afterCellResizeBytes, typedBytes);
    assert.equal(history.entryLabels().undo.length, 2);
    assert.equal(history.entryLabels().undo[1].type, 'snapshot:resizeTableCells');

    resizeTableProportional.call(host, 'ArrowDown');
    assert.equal(history.entryLabels().undo.length, 3);
    assert.equal(history.entryLabels().undo[2].type, 'snapshot:resizeTableCells');

    assert.ok(history.undo(wasm));
    assert.deepEqual(wasm.exportHwpx(), afterCellResizeBytes);
    assert.ok(history.undo(wasm));
    assert.deepEqual(wasm.exportHwpx(), typedBytes);
    assert.equal(stateLabelOf(wasm), typedLabel);

    // Expected-epoch binding & no-op finish on ObjectGestureRecorder
    const expectedEpoch = documentEpochOf(wasm)!;
    const noopRec = new ObjectGestureRecorder('resizeObjects', position(0, 0), expectedEpoch);
    assert.equal(noopRec.begin(wasm), true);
    assert.equal(noopRec.finish(wasm), null, 'no-op gesture discards before snapshot and returns null');
    assert.equal(rawSnapshotLabels(wasm, 4), undefined, 'no-op snapshot id was discarded');

    const staleBoundRec = new ObjectGestureRecorder('moveObjects', position(0, 0), expectedEpoch);
    wasm.loadDocument(fixture(para('<hp:t>FRESH</hp:t>')), 'fresh.hwpx');
    history.clear(wasm);
    assert.equal(staleBoundRec.begin(wasm), false, 'expectedEpoch mismatch refuses begin without saveSnapshot');
    assert.equal(rawSnapshotLabels(wasm, 0), undefined, 'no snapshot allocated in fresh document');
  } finally {
    history.clear(wasm);
    wasm.dispose();
  }
});

type C11ObjectRef = { sec: number; ppi: number; ci: number; type: 'image' | 'shape' | 'group' | 'line' };
const c11GestureKinds = ['single', 'multi', 'group', 'rotate', 'picture-move', 'multi-move', 'table-move', 'line-endpoint', 'table-resize'] as const;

function c11PointerCase(kind: typeof c11GestureKinds[number], dir = 'nw', nativeSizes?: readonly (readonly [number, number])[]) {
  const wasm = new WasmBridge(), history = new CommandHistory();
  wasm.createNewDocument();
  assert.equal(wasm.createTable(0, 0, 0, 2, 2).ok, true);
  const image = wasm.insertPicture(0, 1, 0, c10Png, 3000, 3000, 1, 1, 'png');
  wasm.setPictureProperties(0, image.paraIdx, image.controlIdx, { treatAsChar: false, horzOffset: 1500, vertOffset: 1500 });
  let refs: C11ObjectRef[] = [{ sec: 0, ppi: image.paraIdx, ci: image.controlIdx, type: 'image' }];
  if (nativeSizes || kind === 'multi' || kind === 'multi-move' || kind === 'group' || kind === 'line-endpoint') {
    refs = (kind === 'line-endpoint' ? [1500] : [7500, 12000]).map((horzOffset, index) => {
      const shape = wasm.createShapeControl({ sectionIdx: 0, paraIdx: wasm.getParagraphCount(0) - 1, charOffset: 0,
        width: nativeSizes?.[index]?.[0] ?? 3000, height: nativeSizes?.[index]?.[1] ?? 1500,
        horzOffset: nativeSizes ? 1523 : horzOffset, vertOffset: nativeSizes ? 1571 : 1500,
        shapeType: kind === 'line-endpoint' ? 'line' : 'rectangle', treatAsChar: false });
      assert.equal(shape.ok, true);
      return { sec: 0, ppi: shape.paraIdx, ci: shape.controlIdx, type: kind === 'line-endpoint' ? 'line' : 'shape' };
    });
    if (kind === 'group') {
      const group = wasm.groupShapes(0, refs.map(r => ({ paraIdx: r.ppi, controlIdx: r.ci })));
      assert.equal(group.ok, true);
      refs = [{ sec: 0, ppi: group.paraIdx, ci: group.controlIdx, type: 'group' }];
    }
    if (kind === 'single') refs = refs.slice(0, 1);
  }
  const ref = refs[0];
  assert.ok(ref);
  wasm.insertText(0, wasm.getParagraphCount(0) - 1, 0, 'AB');
  const table = wasm.getPageControlLayout(0).controls.find(control => control.type === 'table');
  if (!table || table.secIdx === undefined || table.paraIdx === undefined || table.controlIdx === undefined) throw new Error('C11 fixture table has no layout');
  const tableRef = { sec: table.secIdx, ppi: table.paraIdx, ci: table.controlIdx };
  const changes = { count: 0 };
  const sc = { clientWidth: 1000, getBoundingClientRect: () => ({ left: 0, top: 0 }) };
  const host = {
    active: true, wasm, history, dragRafId: 0,
    container: { style: { cursor: '' }, querySelector: () => sc },
    viewportManager: { getZoom: () => 1 },
    virtualScroll: { getPageOffset: () => 0, getPageWidth: () => 800, getPageLeft: () => 0 },
    pictureObjectRenderer: { renderDragPreview() {}, clearDragPreview() {} },
    tableResizeRenderer: { showDragMarker() {}, clear() {} },
    eventBus: { emit(name: string) { if (name === 'document-changed') changes.count++; } },
    onMouseUpBound() {},
  };
  const state: Record<string, unknown> & { recorder?: ObjectGestureRecorder } = { epoch: documentEpochOf(wasm), beforeLabel: stateLabelOf(wasm) };
  let clientX = 100, clientY = 100;
  switch (kind) {
    case 'single': case 'multi': case 'group': {
      const items = refs.map(r => {
        const props = getObjectProperties.call(host, r), bbox = findPictureBbox.call(host, r);
        if (!bbox) throw new Error('C11 fixture object has no layout');
        return { ...r, origWidth: props.width, origHeight: props.height, origHorzOffset: props.horzOffset,
          origVertOffset: props.vertOffset, bboxX: bbox.x, bboxY: bbox.y, w: bbox.w, h: bbox.h, pageIndex: bbox.pageIndex };
      });
      const first = items[0];
      assert.ok(first);
      const x = Math.min(...items.map(r => r.bboxX)), y = Math.min(...items.map(r => r.bboxY));
      const w = Math.max(...items.map(r => r.bboxX + r.w)) - x, h = Math.max(...items.map(r => r.bboxY + r.h)) - y;
      Object.assign(state, { dir, ref, origWidth: Math.round(w * 75), origHeight: Math.round(h * 75),
        startClientX: clientX, startClientY: clientY, lastClientX: clientX, lastClientY: clientY,
        pageIndex: first.pageIndex, rotationAngle: 0, bbox: { x, y, w, h }, ...(kind === 'multi' ? { multiRefs: items } : {}) });
      Object.assign(host, { isPictureResizeDragging: true, pictureResizeState: state });
      break;
    }
    case 'rotate':
      Object.assign(state, { ref, origAngle: 0, centerX: 50, centerY: 100, startAngle: 0, pageIndex: 0 });
      Object.assign(host, { isPictureRotateDragging: true, pictureRotateState: state });
      break;
    case 'picture-move': case 'multi-move':
      Object.assign(state, { ref, pageIndex: 0, lastPageX: clientX, lastPageY: clientY, totalDeltaH: 0, totalDeltaV: 0,
        ...(kind === 'multi-move' ? { multiRefs: refs } : {}) });
      Object.assign(host, { isPictureMoveDragging: true, pictureMoveState: state });
      break;
    case 'table-move':
      Object.assign(state, { tableRef, startPpi: tableRef.ppi, pageIndex: 0,
        lastPageX: clientX, lastPageY: clientY, totalDeltaH: 0, totalDeltaV: 0 });
      Object.assign(host, { isMoveDragging: true, moveDragState: state });
      break;
    case 'line-endpoint': {
      const bbox = findPictureBbox.call(host, ref);
      if (!bbox || bbox.x2 === undefined || bbox.y2 === undefined) throw new Error('C11 fixture line has no endpoint');
      clientX = bbox.x2; clientY = bbox.y2;
      Object.assign(state, { ref, endpoint: 'end', pageIndex: 0, pageLeft: 0, pageOffset: 0, zoom: 1 });
      Object.assign(host, { isLineEndpointDragging: true, lineEndpointState: state });
      break;
    }
    case 'table-resize': {
      const boxes = wasm.getTableCellBboxes(tableRef.sec, tableRef.ppi, tableRef.ci);
      const borders = TableResizeRenderer.prototype.computeBorderLines(boxes);
      const border = borders.colLines[1];
      assert.ok(border);
      clientX = border.x;
      Object.assign(state, { tableRef, edge: { type: 'col', index: border.index, pageIndex: 0 },
        borderOriginalPos: clientX, pageBboxes: boxes, lastClientX: clientX, lastClientY: clientY });
      Object.assign(host, { isResizeDragging: true, resizeDragState: state });
      break;
    }
    default: { const unreachable: never = kind; throw new RangeError(unreachable); }
  }
  return { wasm, history, host, state, changes, clientX, clientY, refs, canExport: refs.every(r => r.type === 'image') };
}

test('C11 zero-motion pointer move and mouseup or interrupt preserve redo across every gesture family', () => {
  for (const kind of c11GestureKinds) for (const terminal of ['mouseup', 'interrupt'] as const) for (const delta of [0, 0.001]) {
    withPointerHarness(({ flush }) => {
      const c = c11PointerCase(kind);
      const { wasm, history, host, state, changes, clientX, clientY } = c;
      try {
        const target = wasm.getParagraphCount(0) - 1;
        history.execute(new InsertTextCommand(position(target, 2), 'Z', 1000), wasm);
        const typed = fullSemanticState(wasm, c.canExport);
        assert.ok(history.undo(wasm));
        const before = fullSemanticState(wasm, c.canExport), label = stateLabelOf(wasm), entries = history.entryLabels();
        assert.equal(history.canRedo(), true);
        Reflect.apply(onMouseMove, host, [{ clientX: clientX + delta, clientY: clientY + delta, buttons: 1, ctrlKey: false, shiftKey: false }]);
        flush();
        assert.equal(Boolean(state.recorder?.started), false, `${kind}/${terminal}: no recorder at zero motion`);
        if (terminal === 'mouseup') Reflect.apply(onMouseUp, host, [{ clientX: clientX + delta, clientY: clientY + delta, buttons: 0 }]);
        else Reflect.apply(InputHandler.prototype.settleOpenPointerGesture, host, ['interrupt']);
        assert.equal(hasOpenPointerGesture(host), false);
        assert.deepEqual(fullSemanticState(wasm, c.canExport), before, `${kind}/${terminal}: no document edit`);
        assert.equal(stateLabelOf(wasm), label, `${kind}/${terminal}: no native label advance`);
        assert.deepEqual(history.entryLabels(), entries, `${kind}/${terminal}: history unchanged`);
        assert.equal(changes.count, 0, `${kind}/${terminal}: no document-changed event`);
        assert.equal(history.canRedo(), true);
        assert.ok(history.redo(wasm), `${kind}/${terminal}: redo remains applicable`);
        assert.deepEqual(fullSemanticState(wasm, c.canExport), typed, `${kind}/${terminal}: redo restores typed state`);
      } finally { history.clear(wasm); wasm.dispose(); }
    });
  }
  console.log('C11_ZERO_MOTION gestures=9 terminals=2 deltas=2 cases=36 redo_preserved=36/36');
});

test('C11 resize drag out and return restores live geometry and keeps native label history connected', () => {
  for (const kind of ['single', 'multi', 'group'] as const) for (const terminal of ['mouseup', 'interrupt'] as const) {
    withPointerHarness(({ flush }) => {
      const c = c11PointerCase(kind), { wasm, history, host, clientX, clientY } = c;
      try {
        const target = wasm.getParagraphCount(0) - 1;
        history.execute(new InsertTextCommand(position(target, 2), 'Z', 1000), wasm);
        const before = fullSemanticState(wasm, c.canExport), label = stateLabelOf(wasm);
        for (const [dx, dy] of [[-20, -10], [0, 0]]) {
          Reflect.apply(onMouseMove, host, [{ clientX: clientX + dx, clientY: clientY + dy, buttons: 1 }]);
          flush();
        }
        assert.deepEqual(fullSemanticState(wasm, c.canExport), before, `${kind}/${terminal}: live return geometry`);
        if (terminal === 'mouseup') Reflect.apply(onMouseUp, host, [{ clientX, clientY, buttons: 0 }]);
        else Reflect.apply(InputHandler.prototype.settleOpenPointerGesture, host, ['interrupt']);
        assert.deepEqual(fullSemanticState(wasm, c.canExport), before, `${kind}/${terminal}: settled return geometry`);
        const liveLabel = stateLabelOf(wasm);
        assert.equal(history.entryLabels().undo.length, kind === 'single' ? 1 : 2);
        if (kind !== 'single') {
          assert.notEqual(liveLabel, label, 'a real live edit keeps its new label even when geometry returns');
          assert.ok(history.undo(wasm));
          assert.equal(stateLabelOf(wasm), label);
        }
        assert.ok(history.undo(wasm), `${kind}/${terminal}: older typing remains undoable`);
        assert.equal(wasm.getTextRange(0, target, 0, 3), 'AB');
        assert.ok(history.redo(wasm));
        if (kind !== 'single') assert.ok(history.redo(wasm));
        assert.equal(stateLabelOf(wasm), liveLabel);
        assert.deepEqual(fullSemanticState(wasm, c.canExport), before);
      } finally { history.clear(wasm); wasm.dispose(); }
    });
  }
});

test('C11 multi resize finalization preserves independent corner and edge scale from the live update', () => {
  for (const dir of ['nw', 'se', 'e', 'n']) withPointerHarness(({ flush }) => {
    const { wasm, history, host, clientX, clientY } = c11PointerCase('multi', dir);
    try {
      Reflect.apply(onMouseMove, host, [{ clientX: clientX + 20, clientY: clientY + 5, buttons: 1 }]);
      flush();
      const live = fullSemanticState(wasm, false);
      Reflect.apply(onMouseUp, host, [{ clientX: clientX + 20, clientY: clientY + 5, buttons: 0 }]);
      assert.deepEqual(fullSemanticState(wasm, false), live, `${dir}: mouseup must retain the previewed geometry`);
      assert.equal(history.entryLabels().undo.length, 1);
    } finally { history.clear(wasm); wasm.dispose(); }
  });
});

function c12NativeResizeCase(kind: 'single' | 'multi' | 'group', dir: string, smallSize?: number | readonly (readonly [number, number])[]) {
  const sizes = typeof smallSize === 'number' ? [[smallSize, smallSize], [smallSize, smallSize]] as const : smallSize;
  const c = c11PointerCase(kind, dir, sizes);
  const separated = smallSize === undefined;
  c.refs.forEach((ref, index) => setObjectProperties.call(c.host, ref, {
    ...(smallSize === undefined ? { width: kind === 'group' ? 9512 : 3001 + index * 1008,
      height: kind === 'group' ? 3888 : 1507 + index * 504 } : {}),
    horzOffset: 1523 + (separated ? index * 5503 : 0), vertOffset: 1571 + (separated ? index * 1877 : 0),
  }));
  const items = c.refs.map(ref => {
    const props: { width: number; height: number; horzOffset: number; vertOffset: number } = getObjectProperties.call(c.host, ref);
    const bbox = findPictureBbox.call(c.host, ref);
    if (!bbox) throw new RangeError('C12 native object has no layout');
    return { ...ref, origWidth: props.width, origHeight: props.height, origHorzOffset: props.horzOffset,
      origVertOffset: props.vertOffset, bboxX: bbox.x, bboxY: bbox.y, w: bbox.w, h: bbox.h };
  });
  const first = items[0];
  assert.ok(first);
  const x = Math.min(...items.map(item => item.bboxX)), y = Math.min(...items.map(item => item.bboxY));
  const w = Math.max(...items.map(item => item.bboxX + item.w)) - x;
  const h = Math.max(...items.map(item => item.bboxY + item.h)) - y;
  Object.assign(c.state, { origWidth: kind === 'multi' ? Math.round(w * 75) : first.origWidth,
    origHeight: kind === 'multi' ? Math.round(h * 75) : first.origHeight,
    origHorzOffset: first.origHorzOffset, origVertOffset: first.origVertOffset, bbox: { x, y, w, h },
    ...(kind === 'multi' ? { multiRefs: items } : {}) });
  return { ...c, items, bbox: { x, y, w, h } };
}

test('C12 fractional native resize geometry and redo survive zero or rounded-to-zero motion on every handle', () => {
  let cases = 0;
  for (const kind of ['single', 'multi', 'group'] as const) for (const dir of ['n', 's', 'e', 'w', 'ne', 'nw', 'se', 'sw'])
    for (const terminal of ['mouseup', 'interrupt'] as const) for (const zoom of [0.67, 1.75]) {
      withPointerHarness(({ flush }) => {
        const c = c12NativeResizeCase(kind, dir), { wasm, history, host, state, clientX, clientY } = c;
        try {
          host.viewportManager.getZoom = () => zoom;
          delete state.origHorzOffset; delete state.origVertOffset;
          const target = wasm.getParagraphCount(0) - 1;
          history.execute(new InsertTextCommand(position(target, 2), 'Z', 1000), wasm);
          const typed = fullSemanticState(wasm, false);
          assert.ok(history.undo(wasm));
          const before = fullSemanticState(wasm, false), label = stateLabelOf(wasm), entries = history.entryLabels();
          for (const [dx, dy] of [[0, 0], [-0.001, 0.001], [0.001, -0.001]]) {
            Reflect.apply(onMouseMove, host, [{ clientX: clientX + dx, clientY: clientY + dy, buttons: 1 }]); flush();
          }
          assert.equal(Boolean(state.recorder?.started), false, `${kind}/${dir}: no recorder for zero HWP delta`);
          if (terminal === 'mouseup') Reflect.apply(onMouseUp, host, [{ clientX: clientX + 0.001, clientY: clientY - 0.001, buttons: 0 }]);
          else Reflect.apply(InputHandler.prototype.settleOpenPointerGesture, host, ['interrupt']);
          assert.deepEqual(fullSemanticState(wasm, false), before, `${kind}/${dir}/${terminal}/${zoom}`);
          assert.equal(stateLabelOf(wasm), label);
          assert.deepEqual(history.entryLabels(), entries);
          assert.equal(c.changes.count, 0);
          assert.ok(history.redo(wasm));
          assert.deepEqual(fullSemanticState(wasm, false), typed);
          cases++;
        } finally { history.clear(wasm); wasm.dispose(); }
      });
    }
  console.log(`C12_NATIVE_IDENTITY cases=${cases} redo_preserved=${cases}/${cases}`);
});

test('C12 native baseline plus pointer delta preserves orthogonal dimensions and offsets on edges and corners', () => {
  for (const kind of ['single', 'multi', 'group'] as const) for (const dir of ['e', 'n', 'nw', 'se']) for (const size of [undefined, 100, 30, 3, 1]) {
    withPointerHarness(({ flush }) => {
      const c = c12NativeResizeCase(kind, dir, size), { wasm, history, host, clientX, clientY } = c;
      try {
        const before = fullSemanticState(wasm, false);
        const dx = size === undefined ? 2 : dir.includes('w') ? -4 : 4;
        const dy = size === undefined ? 3 : dir.includes('n') ? -5 : 5;
        const dw = dir.includes('e') ? dx : dir.includes('w') ? -dx : 0;
        const dh = dir.includes('s') ? dy : dir.includes('n') ? -dy : 0;
        const nativeW = Math.max(...c.items.map(item => item.origWidth)) / 75;
        const nativeH = Math.max(...c.items.map(item => item.origHeight)) / 75;
        const sx = 1 + dw / (c.bbox.w || nativeW), sy = 1 + dh / (c.bbox.h || nativeH);
        Reflect.apply(onMouseMove, host, [{ clientX: clientX + dx, clientY: clientY + dy, buttons: 1 }]); flush();
        const live = fullSemanticState(wasm, false);
        Reflect.apply(onMouseUp, host, [{ clientX: clientX + dx, clientY: clientY + dy, buttons: 0 }]);
        if (kind !== 'single') assert.deepEqual(fullSemanticState(wasm, false), live);
        for (const item of c.items) {
          const props = getObjectProperties.call(host, item);
          assert.equal(props.width, Math.max(Math.min(283, item.origWidth), item.origWidth + Math.round(kind === 'multi' ? item.origWidth * (sx - 1) : dw * 75)));
          assert.equal(props.height, Math.max(Math.min(283, item.origHeight), item.origHeight + Math.round(kind === 'multi' ? item.origHeight * (sy - 1) : dh * 75)));
          assert.equal(props.horzOffset, item.origHorzOffset + Math.round(((dir.includes('w') ? dx : 0)
            + (kind === 'multi' ? (item.bboxX - c.bbox.x) * (sx - 1) : 0)) * 75));
          assert.equal(props.vertOffset, item.origVertOffset + Math.round(((dir.includes('n') ? dy : 0)
            + (kind === 'multi' ? (item.bboxY - c.bbox.y) * (sy - 1) : 0)) * 75));
        }
        const after = fullSemanticState(wasm, false);
        assert.ok(history.undo(wasm)); assert.deepEqual(fullSemanticState(wasm, false), before);
        assert.ok(history.redo(wasm)); assert.deepEqual(fullSemanticState(wasm, false), after);
      } finally { history.clear(wasm); wasm.dispose(); }
    });
  }
});

test('C12 fractional native drag out and back restores geometry and keeps older undo and redo labels connected', () => {
  for (const kind of ['single', 'multi', 'group'] as const) for (const dir of ['n', 's', 'e', 'w', 'ne', 'sw'])
    for (const terminal of ['mouseup', 'interrupt'] as const) withPointerHarness(({ flush }) => {
      const c = c12NativeResizeCase(kind, dir), { wasm, history, host, state, clientX, clientY } = c;
      try {
        delete state.origHorzOffset; delete state.origVertOffset;
        const target = wasm.getParagraphCount(0) - 1, initial = fullSemanticState(wasm, false);
        history.execute(new InsertTextCommand(position(target, 2), 'K', 1000), wasm);
        const older = fullSemanticState(wasm, false);
        history.execute(new InsertTextCommand(position(target, 3), 'Z', 3000), wasm);
        assert.ok(history.undo(wasm));
        const beforeLabel = stateLabelOf(wasm);
        for (const [dx, dy] of [[17.23, -9.87], [-11.09, 15.31], [0, 0]]) {
          Reflect.apply(onMouseMove, host, [{ clientX: clientX + dx, clientY: clientY + dy, buttons: 1 }]); flush();
        }
        assert.deepEqual(fullSemanticState(wasm, false), older, `${kind}/${dir}: live return`);
        if (terminal === 'mouseup') Reflect.apply(onMouseUp, host, [{ clientX, clientY, buttons: 0 }]);
        else Reflect.apply(InputHandler.prototype.settleOpenPointerGesture, host, ['interrupt']);
        assert.deepEqual(fullSemanticState(wasm, false), older, `${kind}/${dir}: final return`);
        const afterLabel = stateLabelOf(wasm);
        assert.equal(history.entryLabels().undo.length, kind === 'single' ? 1 : 2);
        assert.equal(history.canRedo(), kind === 'single');
        if (kind !== 'single') { assert.ok(history.undo(wasm)); assert.equal(stateLabelOf(wasm), beforeLabel); }
        assert.ok(history.undo(wasm)); assert.deepEqual(fullSemanticState(wasm, false), initial);
        assert.ok(history.redo(wasm));
        if (kind !== 'single') assert.ok(history.redo(wasm));
        assert.equal(stateLabelOf(wasm), afterLabel);
        assert.deepEqual(fullSemanticState(wasm, false), older);
      } finally { history.clear(wasm); wasm.dispose(); }
    });
});

test('C12 valid subminimum and subpixel rotated shapes retain native dimensions on zero and orthogonal resize', () => {
  for (const size of [100, 30, 3, 1]) for (const kind of ['single', 'multi', 'group'] as const)
    for (const angle of kind === 'multi' ? [0] : [0, 45]) for (const terminal of ['mouseup', 'interrupt'] as const) {
      withPointerHarness(({ flush }) => {
        const c = c12NativeResizeCase(kind, 'e', size), { wasm, history, host, state, clientX, clientY } = c;
        try {
          if (angle !== 0) setObjectProperties.call(host, c.refs[0], { rotationAngle: angle });
          state.rotationAngle = angle;
          const target = wasm.getParagraphCount(0) - 1;
          history.execute(new InsertTextCommand(position(target, 2), 'Z', 1000), wasm);
          assert.ok(history.undo(wasm));
          const before = fullSemanticState(wasm, false), label = stateLabelOf(wasm);
          const radians = angle * Math.PI / 180;
          const dx = -10 * Math.sin(radians), dy = 10 * Math.cos(radians);
          for (const [mx, my] of [[0, 0], [0.001, -0.001], [dx, dy]]) {
            Reflect.apply(onMouseMove, host, [{ clientX: clientX + mx, clientY: clientY + my, buttons: 1 }]); flush();
          }
          assert.equal(Boolean(state.recorder?.started), false);
          if (terminal === 'mouseup') Reflect.apply(onMouseUp, host, [{ clientX: clientX + dx, clientY: clientY + dy, buttons: 0 }]);
          else Reflect.apply(InputHandler.prototype.settleOpenPointerGesture, host, ['interrupt']);
          assert.deepEqual(fullSemanticState(wasm, false), before, `${size}/${kind}/${angle}/${terminal}`);
          assert.equal(stateLabelOf(wasm), label);
          assert.ok(history.canRedo());
        } finally { history.clear(wasm); wasm.dispose(); }
      });
    }
});

test('C13 genuine sub-200 shapes and groups retain untouched axes through live and final edge resize and exact undo redo', () => {
  const asymmetric = [[1, 3], [3, 1]] as const;
  let cases = 0;
  for (const kind of ['single', 'multi', 'group'] as const) for (const order of [0, 1] as const)
    for (const dir of ['e', 'w', 'n', 's']) for (const zoom of [0.41, 2.625])
      for (const terminal of ['mouseup', 'interrupt'] as const) withPointerHarness(({ flush }) => {
        const sizes = kind === 'group' ? [asymmetric[order], asymmetric[order]] : order === 0 ? asymmetric : [asymmetric[1], asymmetric[0]];
        const c = c12NativeResizeCase(kind, dir, sizes), { wasm, history, host, clientX, clientY } = c;
        try {
          host.viewportManager.getZoom = () => zoom;
          c.items.forEach((item, index) => {
            assert.equal(item.origWidth, sizes[index]?.[0]);
            assert.equal(item.origHeight, sizes[index]?.[1]);
          });
          const before = fullSemanticState(wasm, false);
          const dx = zoom * (dir === 'w' ? -12 : dir === 'e' ? 12 : 0);
          const dy = zoom * (dir === 'n' ? -12 : dir === 's' ? 12 : 0);
          Reflect.apply(onMouseMove, host, [{ clientX: clientX + dx, clientY: clientY + dy, buttons: 1 }]); flush();
          const checkAxes = () => c.items.forEach(item => {
            const props = getObjectProperties.call(host, item);
            assert.equal(dir === 'e' || dir === 'w' ? props.height : props.width,
              dir === 'e' || dir === 'w' ? item.origHeight : item.origWidth, `${kind}/${order}/${dir}: untouched axis`);
          });
          checkAxes();
          if (terminal === 'mouseup') Reflect.apply(onMouseUp, host, [{ clientX: clientX + dx, clientY: clientY + dy, buttons: 0 }]);
          else Reflect.apply(InputHandler.prototype.settleOpenPointerGesture, host, ['interrupt']);
          checkAxes();
          const after = fullSemanticState(wasm, false);
          assert.notDeepEqual(after, before);
          assert.equal(history.entryLabels().undo.length, 1);
          assert.ok(history.undo(wasm)); assert.deepEqual(fullSemanticState(wasm, false), before);
          assert.ok(history.redo(wasm)); assert.deepEqual(fullSemanticState(wasm, false), after);
          cases++;
        } finally { history.clear(wasm); wasm.dispose(); }
      });
  console.log(`C13_SELECTIVE_AXES cases=${cases} exact_undo_redo=${cases}/${cases}`);
});

test('C13 sub-200 out and back restores exact native snapshots and keeps earlier history connected', () => {
  const asymmetric = [[1, 3], [3, 1]] as const;
  let cases = 0;
  for (const kind of ['single', 'multi', 'group'] as const) for (const order of [0, 1] as const)
    for (const dir of ['n', 's', 'e', 'w', 'nw', 'ne', 'sw', 'se'])
      for (const terminal of ['mouseup', 'interrupt'] as const) withPointerHarness(({ flush }) => {
        const sizes = kind === 'group' ? [asymmetric[order], asymmetric[order]] : order === 0 ? asymmetric : [asymmetric[1], asymmetric[0]];
        const c = c12NativeResizeCase(kind, dir, sizes), { wasm, history, host, state, clientX, clientY } = c;
        try {
          const target = wasm.getParagraphCount(0) - 1, initial = fullSemanticState(wasm, false);
          history.execute(new InsertTextCommand(position(target, 2), 'K', 1000), wasm);
          const before = fullSemanticState(wasm, false), label = stateLabelOf(wasm);
          history.execute(new InsertTextCommand(position(target, 3), 'Z', 3000), wasm);
          assert.ok(history.undo(wasm));
          const entries = history.entryLabels();
          const dx = dir.includes('w') ? -12 : 12, dy = dir.includes('n') ? -12 : 12;
          for (const [mx, my] of [[dx, dy], [-dx / 3, -dy / 2], [0, 0]]) {
            Reflect.apply(onMouseMove, host, [{ clientX: clientX + mx, clientY: clientY + my, buttons: 1 }]); flush();
          }
          assert.deepEqual(fullSemanticState(wasm, false), before, `${kind}/${order}/${dir}: live return`);
          if (terminal === 'mouseup') Reflect.apply(onMouseUp, host, [{ clientX, clientY, buttons: 0 }]);
          else Reflect.apply(InputHandler.prototype.settleOpenPointerGesture, host, ['interrupt']);
          assert.deepEqual(fullSemanticState(wasm, false), before, `${kind}/${order}/${dir}: final return`);
          if (kind === 'single') {
            assert.deepEqual(history.entryLabels(), entries);
            assert.equal(stateLabelOf(wasm), label);
          } else {
            assert.equal(Boolean(state.recorder?.started), false);
            assert.equal(history.entryLabels().undo.length, 2);
            assert.equal(history.canRedo(), false);
            assert.ok(history.undo(wasm)); assert.equal(stateLabelOf(wasm), label);
          }
          assert.ok(history.undo(wasm)); assert.deepEqual(fullSemanticState(wasm, false), initial);
          assert.ok(history.redo(wasm));
          if (kind !== 'single') assert.ok(history.redo(wasm));
          assert.deepEqual(fullSemanticState(wasm, false), before);
          cases++;
        } finally { history.clear(wasm); wasm.dispose(); }
      });
  console.log(`C13_SNAPSHOT_RETURN cases=${cases} history_connected=${cases}/${cases}`);
});

test('C13 inward resize blocked at the native floor keeps the rotated pivot label and redo unchanged', () => {
  let cases = 0;
  for (const kind of ['single', 'group'] as const) for (const angle of kind === 'single' ? [0, 37, 137, 271] : [0])
    for (const size of [200, 283]) for (const dir of ['n', 's', 'e', 'w', 'nw', 'ne', 'sw', 'se'])
      for (const terminal of ['mouseup', 'interrupt'] as const) withPointerHarness(({ flush }) => {
        const c = c12NativeResizeCase(kind, dir, size), { wasm, history, host, state, clientX, clientY } = c;
        try {
          if (kind === 'single') setObjectProperties.call(host, c.refs[0], { rotationAngle: angle });
          state.rotationAngle = angle;
          const target = wasm.getParagraphCount(0) - 1;
          history.execute(new InsertTextCommand(position(target, 2), 'Z', 1000), wasm);
          const typed = fullSemanticState(wasm, false);
          assert.ok(history.undo(wasm));
          const before = fullSemanticState(wasm, false), label = stateLabelOf(wasm), entries = history.entryLabels();
          const rad = angle * Math.PI / 180;
          const x = dir.includes('w') ? 0.6 : dir.includes('e') ? -0.6 : 0;
          const y = dir.includes('n') ? 0.6 : dir.includes('s') ? -0.6 : 0;
          const dx = x * Math.cos(rad) - y * Math.sin(rad), dy = x * Math.sin(rad) + y * Math.cos(rad);
          Reflect.apply(onMouseMove, host, [{ clientX: clientX + dx, clientY: clientY + dy, buttons: 1 }]); flush();
          assert.equal(Boolean(state.recorder?.started), false);
          if (terminal === 'mouseup') Reflect.apply(onMouseUp, host, [{ clientX: clientX + dx, clientY: clientY + dy, buttons: 0 }]);
          else Reflect.apply(InputHandler.prototype.settleOpenPointerGesture, host, ['interrupt']);
          assert.deepEqual(fullSemanticState(wasm, false), before, `${kind}/${angle}/${size}/${dir}/${terminal}`);
          assert.equal(stateLabelOf(wasm), label);
          assert.deepEqual(history.entryLabels(), entries);
          assert.equal(c.changes.count, 0);
          assert.ok(history.redo(wasm)); assert.deepEqual(fullSemanticState(wasm, false), typed);
          cases++;
        } finally { history.clear(wasm); wasm.dispose(); }
      });
  console.log(`C13_CLAMPED_PIVOT cases=${cases} redo_preserved=${cases}/${cases}`);
});

test('C14 changed sub-200 dimensions keep the opposite native edge fixed after crossing the setter floor', () => {
  let cases = 0;
  for (const kind of ['single', 'multi'] as const) for (const dir of ['w', 'n'])
    for (const zoom of [0.41, 2.625]) for (const terminal of ['mouseup', 'interrupt'] as const)
      withPointerHarness(({ flush }) => {
        const c = c12NativeResizeCase(kind, dir, [[1, 3], [3, 1]]), { wasm, history, host, clientX, clientY } = c;
        try {
          host.viewportManager.getZoom = () => zoom;
          const before = fullSemanticState(wasm, false);
          const anchorBefore = Math.max(...c.items.map(item => dir === 'w'
            ? item.origHorzOffset + item.origWidth : item.origVertOffset + item.origHeight));
          const previews: { width: number; height: number }[] = [];
          Object.assign(host.pictureObjectRenderer, { renderDragPreview(box: { width: number; height: number }) { previews.push(box); } });
          for (const distance of [12, 0.8]) {
            Reflect.apply(onMouseMove, host, [{ clientX: clientX - (dir === 'w' ? distance * zoom : 0),
              clientY: clientY - (dir === 'n' ? distance * zoom : 0), buttons: 1 }]); flush();
            if (kind === 'multi' && distance === 12) for (const item of c.items) {
              const props = getObjectProperties.call(host, item);
              assert.ok((dir === 'w' ? props.width : props.height) > 200);
            }
          }
          if (terminal === 'mouseup') Reflect.apply(onMouseUp, host, [{ clientX: clientX - (dir === 'w' ? 0.8 * zoom : 0),
            clientY: clientY - (dir === 'n' ? 0.8 * zoom : 0), buttons: 0 }]);
          else Reflect.apply(InputHandler.prototype.settleOpenPointerGesture, host, ['interrupt']);
          const actual = c.items.map(item => getObjectProperties.call(host, item));
          const anchorAfter = Math.max(...actual.map(props => dir === 'w' ? props.horzOffset + props.width : props.vertOffset + props.height));
          assert.ok(Math.abs(anchorAfter - anchorBefore) <= 1, `${kind}/${dir}/${zoom}/${terminal}: native anchor moved ${anchorAfter - anchorBefore}`);
          actual.forEach((props, index) => {
            assert.equal(dir === 'w' ? props.width : props.height, 200);
            assert.equal(dir === 'w' ? props.height : props.width, dir === 'w' ? c.items[index]?.origHeight : c.items[index]?.origWidth);
          });
          const preview = previews.at(-1);
          if (!preview) throw new RangeError('C14 resize has no drag preview');
          const nativeExtent = Math.max(...actual.map(props => dir === 'w' ? (props.horzOffset | 0) + props.width : (props.vertOffset | 0) + props.height))
            - Math.min(...actual.map(props => dir === 'w' ? props.horzOffset | 0 : props.vertOffset | 0));
          assert.ok(Math.abs((dir === 'w' ? preview.width : preview.height) * 75 - nativeExtent) <= 1, 'preview matches resolved native extent');
          const after = fullSemanticState(wasm, false);
          assert.ok(history.undo(wasm)); assert.deepEqual(fullSemanticState(wasm, false), before);
          assert.ok(history.redo(wasm)); assert.deepEqual(fullSemanticState(wasm, false), after);
          cases++;
        } finally { history.clear(wasm); wasm.dispose(); }
      });
  console.log(`C14_THRESHOLD cases=${cases} fixed_native_edge=${cases}/${cases}`);
});

test('C14 flipped tiny standard and unchanged-size rectangles keep their native pivot across every rotated handle', () => {
  const pivot = (props: { width: number; height: number; horzOffset: number; vertOffset: number },
    handle: { dir: string; angle: number; flipped: boolean }) => {
    const sign = handle.flipped ? -1 : 1, rad = handle.angle * Math.PI / 180;
    const x = (handle.dir.includes('e') ? -props.width / 2 : handle.dir.includes('w') ? props.width / 2 : 0) * sign;
    const y = (handle.dir.includes('s') ? -props.height / 2 : handle.dir.includes('n') ? props.height / 2 : 0) * sign;
    return [(props.horzOffset | 0) + props.width / 2 + x * Math.cos(rad) - y * Math.sin(rad),
      (props.vertOffset | 0) + props.height / 2 + x * Math.sin(rad) + y * Math.cos(rad)];
  };
  let cases = 0;
  for (const sizes of [[[1, 3], [3, 1]], [[200, 200], [200, 200]], [[3000, 1500], [3000, 1500]]] as const)
    for (const angle of [0, 37, 137, 271]) for (const dir of ['e', 'w', 'n', 's', 'ne', 'nw', 'se', 'sw'])
      for (const terminal of ['mouseup', 'interrupt'] as const) withPointerHarness(({ flush }) => {
        const c = c12NativeResizeCase('single', dir, sizes), { wasm, history, host, state, clientX, clientY } = c;
        try {
          const ref = c.refs[0]; assert.ok(ref);
          setObjectProperties.call(host, ref, { rotationAngle: angle, horzOffset: 23, vertOffset: 31 });
          state.rotationAngle = angle; state.origHorzOffset = 23; state.origVertOffset = 31;
          host.viewportManager.getZoom = () => 1.375;
          const original = getObjectProperties.call(host, ref), before = fullSemanticState(wasm, false), label = stateLabelOf(wasm);
          const localX = dir.includes('e') ? -c.bbox.w - 1.13 : dir.includes('w') ? c.bbox.w + 1.13 : 0;
          const localY = dir.includes('s') ? -c.bbox.h - 1.37 : dir.includes('n') ? c.bbox.h + 1.37 : 0;
          const rad = angle * Math.PI / 180;
          const dx = 1.375 * (localX * Math.cos(rad) - localY * Math.sin(rad));
          const dy = 1.375 * (localX * Math.sin(rad) + localY * Math.cos(rad));
          Reflect.apply(onMouseMove, host, [{ clientX: clientX + dx, clientY: clientY + dy, buttons: 1 }]); flush();
          if (terminal === 'mouseup') Reflect.apply(onMouseUp, host, [{ clientX: clientX + dx, clientY: clientY + dy, buttons: 0 }]);
          else Reflect.apply(InputHandler.prototype.settleOpenPointerGesture, host, ['interrupt']);
          const actual = getObjectProperties.call(host, ref);
          const priorPivot = pivot(original, { dir, angle, flipped: false }), finalPivot = pivot(actual, { dir, angle, flipped: true });
          assert.ok(Math.hypot(finalPivot[0] - priorPivot[0], finalPivot[1] - priorPivot[1]) <= 1,
            `${sizes[0]}/${angle}/${dir}/${terminal}: opposite native pivot moved`);
          if (sizes[0][0] === 200) {
            assert.equal(actual.width, original.width); assert.equal(actual.height, original.height);
            assert.notDeepEqual(actual, original, 'equal dimensions must still cross the pivot');
          }
          const after = fullSemanticState(wasm, false), afterLabel = stateLabelOf(wasm);
          assert.ok(history.undo(wasm)); assert.deepEqual(fullSemanticState(wasm, false), before); assert.equal(stateLabelOf(wasm), label);
          assert.ok(history.redo(wasm)); assert.deepEqual(fullSemanticState(wasm, false), after); assert.equal(stateLabelOf(wasm), afterLabel);
          cases++;
        } finally { history.clear(wasm); wasm.dispose(); }
      });
  console.log(`C14_FLIPPED_PIVOT cases=${cases} fixed_native_pivot=${cases}/${cases}`);
});

test('C14 flipped multi-object floor promotion anchors the edge fixed after crossing zero', () => {
  let cases = 0;
  for (const dir of ['e', 'w', 'n', 's']) for (const terminal of ['mouseup', 'interrupt'] as const)
    withPointerHarness(({ flush }) => {
      const c = c12NativeResizeCase('multi', dir, [[1, 3], [3, 1]]), { wasm, history, host, clientX, clientY } = c;
      try {
        const horizontal = dir === 'e' || dir === 'w';
        const before = fullSemanticState(wasm, false);
        const initialAnchor = dir === 'e' || dir === 's'
          ? Math.min(...c.items.map(item => horizontal ? item.origHorzOffset : item.origVertOffset))
          : Math.max(...c.items.map(item => horizontal ? item.origHorzOffset + item.origWidth : item.origVertOffset + item.origHeight));
        const dx = dir === 'e' ? -1.13 : dir === 'w' ? 1.13 : 0;
        const dy = dir === 's' ? -1.37 : dir === 'n' ? 1.37 : 0;
        Reflect.apply(onMouseMove, host, [{ clientX: clientX + dx, clientY: clientY + dy, buttons: 1 }]); flush();
        if (terminal === 'mouseup') Reflect.apply(onMouseUp, host, [{ clientX: clientX + dx, clientY: clientY + dy, buttons: 0 }]);
        else Reflect.apply(InputHandler.prototype.settleOpenPointerGesture, host, ['interrupt']);
        const actual = c.items.map(item => getObjectProperties.call(host, item));
        const finalAnchor = dir === 'e' || dir === 's'
          ? Math.max(...actual.map(props => horizontal ? (props.horzOffset | 0) + props.width : (props.vertOffset | 0) + props.height))
          : Math.min(...actual.map(props => horizontal ? props.horzOffset | 0 : props.vertOffset | 0));
        assert.ok(Math.abs(finalAnchor - initialAnchor) <= 4, `${dir}/${terminal}: promoted size moved flipped anchor ${finalAnchor - initialAnchor}`);
        const after = fullSemanticState(wasm, false);
        assert.ok(history.undo(wasm)); assert.deepEqual(fullSemanticState(wasm, false), before);
        assert.ok(history.redo(wasm)); assert.deepEqual(fullSemanticState(wasm, false), after);
        cases++;
      } finally { history.clear(wasm); wasm.dispose(); }
    });
  console.log(`C14_MULTI_FLIP cases=${cases} fixed_native_edge=${cases}/${cases}`);
});

test('C10 source guard enforces press-epoch checks, replacement hooks, and recorder settlement across all pointer gestures', () => {
  const engineDir = new URL('../src/engine', import.meta.url).pathname;
  const tableCode = readFileSync(join(engineDir, 'input-handler-table.ts'), 'utf8');
  const pictureCode = readFileSync(join(engineDir, 'input-handler-picture.ts'), 'utf8');
  const mouseCode = readFileSync(join(engineDir, 'input-handler-mouse.ts'), 'utf8');
  const handlerCode = readFileSync(join(engineDir, 'input-handler.ts'), 'utf8');

  // Replacement hooks wrap both loadDocument and createNewDocument without touching wasm-bridge.ts
  assert.match(tableCode, /\['loadDocument',\s*'createNewDocument'\]/);
  assert.match(handlerCode, /_table\.bindPointerHostToWasm\(this\)/);

  // Every pointer update/finish handler validates epoch ownership
  const requiredHandlers: Array<[string, string, RegExp]> = [
    ['input-handler-table.ts', tableCode, /export function updateMoveDrag[\s\S]*?ensureGestureMatchesLiveEpoch\(this\)/],
    ['input-handler-table.ts', tableCode, /export function finishMoveDrag[\s\S]*?isActiveGestureEpochStale\(this\)/],
    ['input-handler-table.ts', tableCode, /export function updateResizeDrag[\s\S]*?ensureGestureMatchesLiveEpoch\(this\)/],
    ['input-handler-table.ts', tableCode, /export function finishResizeDrag[\s\S]*?isActiveGestureEpochStale\(this\)/],
    ['input-handler-picture.ts', pictureCode, /export function updatePictureMoveDrag[\s\S]*?ensureGestureMatchesLiveEpoch\(this\)/],
    ['input-handler-picture.ts', pictureCode, /export function finishPictureMoveDrag[\s\S]*?isActiveGestureEpochStale\(this\)/],
    ['input-handler-picture.ts', pictureCode, /export function updatePictureResizeDrag[\s\S]*?ensureGestureMatchesLiveEpoch\(this\)/],
    ['input-handler-picture.ts', pictureCode, /export function finishPictureResizeDrag[\s\S]*?isActiveGestureEpochStale\(this\)/],
    ['input-handler-picture.ts', pictureCode, /export function updatePictureRotateDrag[\s\S]*?ensureGestureMatchesLiveEpoch\(this\)/],
    ['input-handler-picture.ts', pictureCode, /export function finishPictureRotateDrag[\s\S]*?isActiveGestureEpochStale\(this\)/],
    ['input-handler-mouse.ts', mouseCode, /export function updateLineEndpointDrag[\s\S]*?ensureGestureMatchesLiveEpoch\(this\)/],
    ['input-handler-mouse.ts', mouseCode, /export function finishLineEndpointDrag[\s\S]*?isActiveGestureEpochStale\(this\)/],
  ];
  for (const [file, code, pattern] of requiredHandlers) {
    assert.match(code, pattern, `${file}: missing epoch check for ${pattern}`);
  }

  // settleOpenPointerGesture settles all 6 mutating pointer gesture families on interrupt
  const settleStart = handlerCode.indexOf('settleOpenPointerGesture(kind:');
  assert.ok(settleStart >= 0);
  const settleSlice = handlerCode.slice(settleStart, settleStart + 4500);
  for (const finishName of [
    'finishMoveDrag',
    'finishPictureMoveDrag',
    'finishPictureRotateDrag',
    'finishPictureResizeDrag',
    'finishLineEndpointDrag',
    'finishResizeDrag',
  ]) {
    assert.ok(settleSlice.includes(finishName), `settleOpenPointerGesture missing ${finishName}`);
  }
  for (const recorderOwner of [
    'pictureMoveState?.recorder',
    'pictureResizeState?.recorder',
    'pictureRotateState?.recorder',
    'lineEndpointState?.recorder',
    'resizeDragState?.recorder',
  ]) {
    assert.ok(settleSlice.includes(recorderOwner), `settleOpenPointerGesture missing recorder cancel for ${recorderOwner}`);
  }
  console.log('C10_POINTER_GUARD handlers=12 interrupt_finishers=6 cancel_recorders=5 replacement_hooks=2');
});
