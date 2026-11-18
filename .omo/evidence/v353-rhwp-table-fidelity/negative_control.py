# 수정 전 a34d88ff 사본에 v353 통합 시험을 그대로 돌려 page_break·display_height 실패 수를 기록한다.
import subprocess, json, re, sys, hashlib, datetime, os
base, out_json, cargo = sys.argv[1], sys.argv[2], sys.argv[3]
def run(filt):
    p = subprocess.run([cargo, 'test', '--locked', '--test', 'hwpx_table_fidelity', filt],
                       cwd=base, capture_output=True, text=True)
    text = p.stdout + p.stderr
    m = re.search(r'test result: (\w+)\. (\d+) passed; (\d+) failed', text)
    tests = {t: s for t, s in re.findall(r'^test (\S+) \.\.\. (ok|FAILED)$', text, re.M)}
    panics = re.findall(r"panicked at [^\n]*\n([^\n]*)", text)
    return {'exit': p.returncode, 'compiled': m is not None,
            'passed': int(m.group(2)) if m else 0, 'failed': int(m.group(3)) if m else None,
            'tests': tests, 'panic_first_lines': panics[:12]}
pb = run('page_break'); dh = run('display_height'); wf = run('width_formula')
sha = hashlib.sha256(open(os.path.join(base, 'tests/hwpx_table_fidelity.rs'), 'rb').read()).hexdigest()
res = {'base': 'a34d88ff3bae317ad08d8f0b99504e4395278e6a',
       'base_copy': base, 'base_copy_method': 'git archive a34d88ff | tar -x (src/ byte-identical to the base commit)',
       'test_file_sha256': sha,
       'generated_at': datetime.datetime.now(datetime.timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'),
       'page_break_failed': pb['failed'], 'display_height_failed': dh['failed'], 'width_formula_failed': wf['failed'],
       'page_break': pb, 'display_height': dh, 'width_formula': wf}
json.dump(res, open(out_json, 'w'), ensure_ascii=False, indent=2)
print('page_break', pb['passed'], 'passed', pb['failed'], 'failed', 'compiled', pb['compiled'])
print('display_height', dh['passed'], 'passed', dh['failed'], 'failed', 'compiled', dh['compiled'])
print('width_formula', wf['passed'], 'passed', wf['failed'], 'failed', 'compiled', wf['compiled'])
