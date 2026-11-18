# 수정 전 59c2285 사본에 레인 2 시험(파일·표본 그대로)을 돌려 run_order·two_save·residual 실패 수를 남긴다.
# 사용: python3 -I negative_control.py <저장소> <출력 json> <작업 폴더>
#   작업 폴더에 `git archive 59c2285` 사본을 풀고, 저장소의 새 시험 파일·표본·Cargo.lock 을 복사해 돌린다.
import datetime, hashlib, json, os, re, shutil, subprocess, sys

repo, out_json, work = sys.argv[1], sys.argv[2], sys.argv[3]
BASE = "59c2285efe616211cd147dfa849d2f9f9df767dd"
TEST = "tests/hwpx_save_fidelity_l2.rs"
FIXTURES = "tests/fixtures/save-fidelity-l2"

base = os.path.join(work, "base-59c2285")
if os.path.exists(base):
    shutil.rmtree(base)
os.makedirs(base)
archive = subprocess.run(["git", "-C", repo, "archive", BASE], capture_output=True, check=True).stdout
subprocess.run(["tar", "-x", "-C", base], input=archive, check=True)
# Cargo.lock 은 git 에 없다 — 같은 의존성 버전을 쓰도록 작업 트리 것을 복사한다.
shutil.copy(os.path.join(repo, "Cargo.lock"), os.path.join(base, "Cargo.lock"))
shutil.copy(os.path.join(repo, TEST), os.path.join(base, TEST))
shutil.copytree(os.path.join(repo, FIXTURES), os.path.join(base, FIXTURES))

env = {**os.environ, "CARGO_TARGET_DIR": os.path.join(work, "target-base-59c2285")}


def run(args):
    p = subprocess.run(["cargo", "test", *args], cwd=base, capture_output=True, text=True, env=env)
    text = p.stdout + p.stderr
    m = re.search(r"test result: (\w+)\. (\d+) passed; (\d+) failed", text)
    tests = dict(re.findall(r"^test (\S+) \.\.\. (ok|FAILED|ignored)$", text, re.M))
    panics = re.findall(r"panicked at [^\n]*\n([^\n]*)", text)
    return {"exit": p.returncode, "compiled": m is not None, "result_line": m.group(0) if m else None,
            "tests": tests, "panic_first_lines": [x[:300] for x in panics[:16]]}


l2 = run(["--test", "hwpx_save_fidelity_l2"])
g8 = run(["--lib", "serializer::hwpx::tests::form_attribute_entities_survive_two_hwpx_saves"])


def failed(prefix):
    return sum(1 for name, status in l2["tests"].items() if name.startswith(prefix) and status == "FAILED")


g8_failed = int(g8["tests"].get("serializer::hwpx::tests::form_attribute_entities_survive_two_hwpx_saves") == "FAILED")
res = {
    "base": BASE,
    "base_copy": base,
    "base_copy_method": "git archive 59c2285 | tar -x (src/ byte-identical to the base commit) + work-tree Cargo.lock",
    "test_file_sha256": hashlib.sha256(open(os.path.join(repo, TEST), "rb").read()).hexdigest(),
    "generated_at": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "run_order_failed": failed("run_order_"),
    "two_save_failed": failed("two_save_") + g8_failed,
    "two_save_l2_failed": failed("two_save_"),
    "g8_lib_failed": bool(g8_failed),
    "residual_failed": failed("residual_"),
    "guard_tests_passing_at_base": sorted(n for n, s in l2["tests"].items() if s == "ok"),
    "l2": l2,
    "g8_lib": g8,
}
json.dump(res, open(out_json, "w"), ensure_ascii=False, indent=1)
print("run_order_failed", res["run_order_failed"], "two_save_failed", res["two_save_failed"],
      "residual_failed", res["residual_failed"], "l2", l2["result_line"], "g8", g8["result_line"])
