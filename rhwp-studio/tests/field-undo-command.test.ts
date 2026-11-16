import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { initSync } from '../../pkg/rhwp.js';
import { WasmBridge } from '../src/core/wasm-bridge';
import type { DocumentPosition } from '../src/core/types';
import {
  DeleteSelectionCommand, DeleteTextCommand, InsertLineBreakCommand, InsertTabCommand,
  InsertTextCommand, MergeNextParagraphCommand, MergeParagraphCommand,
  SplitParagraphCommand, SplitParagraphInCellCommand, MergeParagraphInCellCommand,
  MergeNextParagraphInCellCommand,
} from '../src/engine/command';
import type { EditCommand, OperationDescriptor } from '../src/engine/command';
import { CommandHistory } from '../src/engine/history';
import { onCompositionStart, onCompositionEnd, onInput, getTextAt, insertTextAtRaw } from '../src/engine/input-handler-text';

initSync({ module: readFileSync(new URL('../../pkg/rhwp_bg.wasm', import.meta.url)) });

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
