import { spawn, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createWriteStream, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const evidence = dirname(fileURLToPath(import.meta.url));
const root = resolve(evidence, '../../..');
const [label, ...argv] = process.argv.slice(2);
if (!label || !/^[a-z0-9_-]+$/.test(label) || argv.length === 0) {
  throw new Error('Usage: node run.mjs <label> <command> [arguments...]');
}
const git = (...args) => {
  const result = spawnSync('/usr/bin/git', ['-C', root, ...args], { encoding: 'utf8' });
  if (result.status !== 0) throw new Error(result.stderr);
  return result.stdout.trim();
};
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');
const record = {
  label, argv, cwd: root, started_at: new Date().toISOString(),
  head: git('rev-parse', 'HEAD'), tree: git('rev-parse', 'HEAD^{tree}'),
  rust_inputs: Object.fromEntries(git('ls-files', 'src', 'tests').split('\n')
    .filter((path) => path.endsWith('.rs'))
    .map((path) => [path, sha256(readFileSync(resolve(root, path)))])),
};
const logPath = resolve(evidence, `${label}.log`);
const log = createWriteStream(logPath, { flags: 'wx' });
const child = spawn(argv[0], argv.slice(1), { cwd: root, stdio: ['ignore', 'pipe', 'pipe'] });
child.stdout.on('data', (chunk) => log.write(chunk));
child.stderr.on('data', (chunk) => log.write(chunk));
child.on('error', (error) => log.write(`${error.stack}\n`));
child.on('close', (rc, signal) => {
  log.end(() => {
    const bytes = readFileSync(logPath);
    Object.assign(record, {
      finished_at: new Date().toISOString(), rc, signal,
      log: logPath, log_bytes: bytes.length, log_sha256: sha256(bytes),
    });
    writeFileSync(resolve(evidence, `${label}.json`), `${JSON.stringify(record, null, 2)}\n`, { flag: 'wx' });
    process.stdout.write(`${JSON.stringify({ label, rc, signal, log: logPath, log_sha256: record.log_sha256 })}\n`);
    process.stdout.write(bytes.toString().split('\n').slice(-18).join('\n').slice(-4000));
    process.exitCode = rc ?? 1;
  });
});
