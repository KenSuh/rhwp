import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const base = '3033ae22c9031919757be1357a998471e67f1fab';
const run = (argv) => {
  const result = spawnSync(argv[0], argv.slice(1), { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
  return result.stdout;
};
const functions = [
  ['src/parser/body_text.rs', 'parse_ctrl_data_field_name'],
  ['src/parser/control.rs', 'parse_cell_field_name'],
  ['src/parser/control.rs', 'decode_utf16le'],
  ['src/wmf/parser/objects/structure/mod.rs', 'utf16le_bytes_to_string'],
];
const extract = (text, name) => {
  const start = text.indexOf(`fn ${name}(`);
  const end = text.indexOf('\n}', start);
  assert.ok(start >= 0 && end > start, name);
  return `pub ${text.slice(start, end + 2)}`;
};
const modules = ['old', 'new'].map((label) => `mod ${label} {\n${functions.map(([path, name]) =>
  extract(label === 'old' ? run(['/usr/bin/git', 'show', `${base}:${path}`]) : readFileSync(path, 'utf8'), name)
).join('\n')}\n}`).join('\n');
const cases = String.raw`
mod wmf { pub mod parser {
    #[derive(Debug, PartialEq)]
    pub enum ParseError { UnexpectedPattern { cause: String } }
} }
fn compare(bytes: &[u8]) {
    assert_eq!(old::decode_utf16le(bytes), new::decode_utf16le(bytes));
    assert_eq!(old::utf16le_bytes_to_string(bytes), new::utf16le_bytes_to_string(bytes));
    let count = u16::try_from(bytes.len() / 2).unwrap();
    let mut field = vec![0; 10];
    field.extend_from_slice(&count.to_le_bytes());
    field.extend_from_slice(bytes);
    assert_eq!(old::parse_ctrl_data_field_name(&field), new::parse_ctrl_data_field_name(&field));
    let mut cell = vec![0; 15];
    cell.extend_from_slice(&count.to_le_bytes());
    cell.extend_from_slice(bytes);
    assert_eq!(old::parse_cell_field_name(&cell), new::parse_cell_field_name(&cell));
}
fn main() {
    assert_eq!(new::decode_utf16le(&[0x5c, 0xd5, 0x00, 0xae, 0xff]), "\u{d55c}\u{ae00}");
    assert_eq!(new::decode_utf16le(&[0x00, 0xd8]), "\u{fffd}");
    assert_eq!(new::decode_utf16le(&[0x3d, 0xd8, 0x00, 0xde]), "\u{1f600}");
    assert!(new::utf16le_bytes_to_string(&[0xff]).is_err());
    assert!(new::utf16le_bytes_to_string(&[0x00, 0xd8]).is_err());
    assert_eq!(new::utf16le_bytes_to_string(&[]).unwrap(), "");
    compare(&[]);
    compare(&[0xff]);
    for unit in 0..=u16::MAX {
        compare(&unit.to_le_bytes());
        let [lo, hi] = unit.to_le_bytes();
        compare(&[lo, hi, 0xff]);
    }
    compare(&[0x3d, 0xd8, 0x00, 0xde]);
    println!("PASS: 131075 UTF-16 inputs across four real source functions; LE, odd tails, malformed surrogates");
}
`;
const temp = mkdtempSync(join(tmpdir(), 'v318-clippy-semantics-'));
try {
  const rs = join(temp, 'semantics.rs');
  const binary = join(temp, 'semantics');
  writeFileSync(rs, `${modules}\n${cases}`);
  run(['rustup', 'run', '1.99.0', 'rustc', '--edition', '2021', rs, '-o', binary]);
  process.stdout.write(run([binary]));
} finally {
  rmSync(temp, { recursive: true });
}
