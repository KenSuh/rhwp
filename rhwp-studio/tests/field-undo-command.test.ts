import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync, readdirSync } from 'node:fs';
import ts from 'typescript';
import { execFileSync } from 'node:child_process';
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
import { CommandHistory } from '../src/engine/history';
import { InputHandler } from '../src/engine/input-handler';
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

const position = (paragraphIndex: number, charOffset: number): DocumentPosition => ({ sectionIndex: 0, paragraphIndex, charOffset });
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

function discoveredCommandClasses(directory: URL, rawExecutions: string[] = [], inverseSites: string[] = []): string[] {
  const classes: string[] = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = new URL(entry.name + (entry.isDirectory() ? '/' : ''), directory);
    if (entry.isDirectory()) {
      classes.push(...discoveredCommandClasses(path, rawExecutions, inverseSites));
    } else if (entry.name.endsWith('.ts')) {
      const source = ts.createSourceFile(path.pathname, readFileSync(path, 'utf8'), ts.ScriptTarget.Latest, true);
      const visit = (node: ts.Node): void => {
        if (ts.isCallExpression(node) && ts.isPropertyAccessExpression(node.expression)) {
          const call = node.expression;
          if (call.name.text === 'execute' && ts.isNewExpression(call.expression)) {
            rawExecutions.push(`${path.pathname}:${source.getLineAndCharacterOfPosition(node.pos).line + 1}`);
          }
          if (call.name.text === 'beginTextEdit' && entry.name !== 'wasm-bridge.ts') {
            let owner: ts.Node | undefined = node.parent;
            while (owner && !ts.isMethodDeclaration(owner) && !ts.isFunctionDeclaration(owner)) owner = owner.parent;
            inverseSites.push(`${entry.name}:${owner?.name?.getText(source) ?? 'anonymous'}`);
          }
        }
        if (ts.isClassDeclaration(node) && node.heritageClauses?.some(clause =>
          clause.token === ts.SyntaxKind.ImplementsKeyword && clause.types.some(type =>
            ts.isIdentifier(type.expression) && type.expression.text === 'EditCommand'))) {
          if (!node.name) throw new Error(`anonymous EditCommand in ${path.pathname}`);
          classes.push(node.name.text);
        }
        ts.forEachChild(node, visit);
      };
      visit(source);
    }
  }
  return classes.sort();
}

test('every discovered Studio command releases its inverse after execute undo redo and clear', () => {
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
  const rawExecutions: string[] = [], inverseSites: string[] = [];
  const discovered = discoveredCommandClasses(new URL('../src/', import.meta.url), rawExecutions, inverseSites);
  assert.deepEqual([...routes.keys()].sort(), discovered, 'each new EditCommand needs a real lifecycle scenario');
  assert.deepEqual(rawExecutions, [], 'commands must execute through their releasing history owner');
  assert.deepEqual(inverseSites.sort(), ['command.ts:run', 'input-handler-text.ts:onInput', 'input-handler-text.ts:onInput'],
    'new inverse allocation routes need lifecycle coverage');
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
    compositionEditToken: null,
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
