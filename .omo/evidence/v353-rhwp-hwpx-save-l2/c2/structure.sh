#!/usr/bin/env bash
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
for v353_script in .omo/evidence/v353-rhwp-hwpx-save-l2/c1/clean-cargo.sh .omo/evidence/v353-rhwp-hwpx-save-l2/c2/probe.sh .omo/evidence/v353-rhwp-hwpx-save-l2/c2/structure.sh .omo/evidence/v353-rhwp-hwpx-save-l2/c2/fault-bin/git; do
    bash -n "$v353_script"
done
git diff --check
python3 - <<'PY'
import hashlib
import pathlib
import re
import subprocess
from typing import Final

BASE: Final = "7370ec9fdcf89eb4a372143bcb555ddd9bfe0b46"
EVIDENCE: Final = ".omo/evidence/v353-rhwp-hwpx-save-l2/"
HELPER: Final = EVIDENCE + "c1/clean-cargo.sh"
LEDGER: Final = "gates/V353_RHWP_HWPX_SAVE_L2_GATES.md"


def git(*args: str) -> str:
    return subprocess.check_output(["/usr/bin/git", *args], text=True).strip()


paths = git("diff", "--name-only", BASE).splitlines()
paths += git("ls-files", "--others", "--exclude-standard").splitlines()
assert paths and all(p in (HELPER, LEDGER) or p.startswith(EVIDENCE + "c2/") for p in paths)
print("C2_SCOPE_OK", len(set(paths)), "paths")
product_roots = ("src", "tests", "Cargo.toml", "build.rs", ".cargo", "template", "assets", "ttfs")
assert not git("diff", "--name-only", BASE, "--", *product_roots)
tree = git("ls-tree", "-r", BASE, "--", *product_roots)
print("PRODUCT_INPUTS_UNCHANGED", hashlib.sha256(tree.encode()).hexdigest())
assert not git("diff", "--name-only", BASE, "--", "gates/V318_ENGINE_CLIPPY_CORRECTION_GATES.md")
historical = git("diff", "--name-only", BASE, "--", EVIDENCE).splitlines()
assert all(p == HELPER or p.startswith(EVIDENCE + "c2/") for p in historical)
print("HISTORICAL_EVIDENCE_AND_V318_UNCHANGED")
for name in (HELPER, "tests/hwpx_save_fidelity_l2.rs", "Cargo.lock"):
    print("SHA256", name, hashlib.sha256(pathlib.Path(name).read_bytes()).hexdigest())
lock = pathlib.Path("Cargo.lock").read_text()
zip_version = re.search(r'name = "zip"\nversion = "([^"]+)"', lock)
assert zip_version is not None
print("OPTIONAL_LOCK_ZIP_VERSION", zip_version[1])

old = git("show", BASE + ":" + LEDGER)
current = pathlib.Path(LEDGER).read_text()
assert re.findall(r"^  EXPECT: (.*)$", old, re.M) == re.findall(r"^  EXPECT: (.*)$", current, re.M)
print("ALL_11_EXPECTS_UNCHANGED")
for gate in (1, 2, 3, 4, 5, 6, 10, 11):
    pattern = rf"^- \[.\] G{gate}:.*?(?=^- \[.\] G|^## |\Z)"
    before = re.search(pattern, old, re.M | re.S)
    after = re.search(pattern, current, re.M | re.S)
    assert before is not None and after is not None
    before_check = re.search(r"^  CHECK: (.*)$", before[0], re.M)
    check = re.search(r"^  CHECK: (.*)$", after[0], re.M)
    assert before_check is not None and check is not None and before_check[1] == check[1]
    if gate in (1, 5, 10, 11):
        print("COMMAND G" + str(gate), check[1], flush=True)
        result = subprocess.run(["/bin/sh", "-c", check[1]], capture_output=True, text=True, check=False)
        print(result.stdout, end="")
        print(result.stderr, end="")
        assert result.returncode == 0
        expect = re.search(r"^  EXPECT: /(.*)/[ms]*$", after[0], re.M)
        assert expect is not None and re.search(expect[1], result.stdout, re.M)
        print("PASS G" + str(gate))
assert len(re.findall(r"^  ABANDON: G[789] DEFERRED-TO-LANDING", current, re.M)) == 3
assert all(f"- [ ] G{gate}:" in current for gate in (7, 8, 9))
print("G7_G8_G9_DEFERRED_NOT_EXECUTED")
PY
grep -E '^[[:space:]]*Finished ' .omo/evidence/v353-rhwp-hwpx-save-l2/c2/focused-no-lock.log | tail -1
printf 'FINISHED_EXTRACTION_OK_WITH_TRAILING_METADATA\nSTRUCTURE_OK\n'
