# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
# Run: python3 .omo/evidence/v353-rhwp-table-fidelity/c3/measure_base.py
"""Bind an isolated G0 archive and measure the unchanged lane tests against it."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
from typing import Final, TypedDict


class Result(TypedDict):
    exit: int
    compiled: bool
    passed: int
    failed: int
    tests: dict[str, str]


BASE: Final = "01a67b3f77760b5a9dae9fc155d83f8ca84ab529"
REPO: Final = Path.cwd()
OUT: Final = REPO / ".omo/evidence/v353-rhwp-table-fidelity/c3"
SNAPSHOT: Final = Path((OUT / "base-snapshot.txt").read_text().strip()).resolve()

entries = subprocess.run(
    ["/usr/bin/git", "ls-tree", "-rz", BASE],
    check=True, capture_output=True,
).stdout.split(b"\0")
verified = []
for entry in filter(None, entries):
    metadata, name = entry.split(b"\t", 1)
    mode, kind, expected = metadata.split()
    if kind != b"blob":
        continue
    path = SNAPSHOT / os.fsdecode(name)
    data = os.fsencode(os.readlink(path)) if mode == b"120000" else path.read_bytes()
    actual = hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()
    assert actual == expected.decode(), os.fsdecode(name)
    verified.append({"path": os.fsdecode(name), "git_blob": actual})
(OUT / "base-source-binding.json").write_text(json.dumps({
    "base": BASE, "snapshot": str(SNAPSHOT), "verified_files": verified,
}, indent=2) + "\n")

# The measurement tool sees an archive without .git. Bind its HEAD to the verified archive,
# preserving the originally observed null rather than claiming it was a Git checkout.
baseline_path = OUT / "baseline.json"
baseline = json.loads(baseline_path.read_text())
assert baseline["head"] in (None, BASE) and baseline["repo"] == str(SNAPSHOT)
baseline["observed_git_head"] = None
baseline["head"] = BASE
baseline["head_binding"] = "base-source-binding.json: every archived Git blob verified"
baseline_path.write_text(json.dumps(baseline, ensure_ascii=False, indent=2) + "\n")

test = REPO / "tests/hwpx_table_fidelity.rs"
excluded_tests = [
    "width_formula_preserves_declaration_when_absolute_widths_are_unchanged",
    "width_formula_uses_assigned_widths_when_all_columns_are_merged",
]
compatible_test = test.read_text()
for name in excluded_tests:
    declaration = f"#[test]\nfn {name}()"
    assert declaration in compatible_test
    compatible_test = compatible_test.replace(declaration, f"#[test]\n#[cfg(any())]\nfn {name}()", 1)
(SNAPSHOT / "tests/hwpx_table_fidelity.rs").write_text(compatible_test)
shutil.copyfile(REPO / "Cargo.lock", SNAPSHOT / "Cargo.lock")
results: dict[str, Result] = {}
environment = dict(os.environ, CARGO_NET_OFFLINE="true", CARGO_TARGET_DIR=str(SNAPSHOT / "target"))
for group in ["page_break", "display_height", "width_formula"]:
    command = ["cargo", "test", "--offline", "--locked", "--test", "hwpx_table_fidelity", group]
    run = subprocess.run(command, cwd=SNAPSHOT, env=environment, text=True,
                         stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
    (OUT / f"negative-{group}.log").write_text(run.stdout)
    summary = re.search(r"test result: \w+\. (\d+) passed; (\d+) failed", run.stdout)
    assert summary is not None, f"{group}: build failure is not a negative control"
    results[group] = {
        "exit": run.returncode, "compiled": True,
        "passed": int(summary[1]), "failed": int(summary[2]),
        "tests": dict(re.findall(r"^test (\S+) \.\.\. (ok|FAILED)$", run.stdout, re.M)),
    }
    print(group, results[group], flush=True)

negative = {
    "base": BASE, "base_copy": str(SNAPSHOT),
    "base_copy_method": "git archive; all source blobs verified in base-source-binding.json",
    "test_file_sha256": hashlib.sha256(test.read_bytes()).hexdigest(),
    "compiled_test_file_sha256": hashlib.sha256(compatible_test.encode()).hexdigest(),
    "base_only_exclusions": excluded_tests,
    "exclusion_reason": "G6 cases call v353-only set_column_widths; only cfg(any()) added in archive, all G4/G5 tests unchanged",
    "page_break_failed": results["page_break"]["failed"],
    "display_height_failed": results["display_height"]["failed"],
    "width_formula_failed": results["width_formula"]["failed"],
    **results,
}
assert negative["page_break_failed"] > 0 and negative["display_height_failed"] > 0
(OUT / "negative-control.json").write_text(json.dumps(negative, indent=2) + "\n")

environment.update(V353_HANCOM_SET="base", V353_HANCOM_OUT=str(REPO / ".omo/evidence/v353-rhwp-table-fidelity/hancom"))
with (OUT / "hancom-base.log").open("w") as log:
    subprocess.run(["cargo", "test", "--offline", "--locked", "--test", "hwpx_table_fidelity",
                    "write_hancom_review_files", "--", "--ignored", "--nocapture"],
                   cwd=SNAPSHOT, env=environment, stdout=log, stderr=subprocess.STDOUT, check=True)
print("BASE_MEASUREMENTS_BOUND", BASE, len(verified), flush=True)
