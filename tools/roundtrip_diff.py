#!/usr/bin/env python3
# HWPX 원본과 rhwp 왕복본(에디터 경로: 열기 → 저장)의 표·셀 속성 손실을 세는 회귀 하네스.
"""HWPX 표 왕복 손실 하네스 (v353).

분석 도구 `_analysis/tools/roundtrip_diff.py` 를 저장소로 옮겨 쓴 것이다. 원 도구는 정규식으로
`<hp:subList>` 를 문서 순서대로 짝지었기 때문에, 셀 밖 subList(글상자 등)가 하나만 빠져도 뒤 셀이
모두 밀려 거짓 손실을 셌다. 여기서는 XML 을 파싱해 표는 문서 순서로, 셀은 `cellAddr` 로 짝짓는다.

두 가지 방식으로 쓴다.

    # 1) 두 파일 비교 (원 도구와 같은 쓰임)
    python3 tools/roundtrip_diff.py SRC.hwpx DST.hwpx

    # 2) 표본 왕복 (프로브를 빌드해 각 표본을 열고 다시 저장한 뒤 비교)
    python3 tools/roundtrip_diff.py --fixtures tests/fixtures/table-fidelity \
        --cases T1,T2,T6,T7 --summary [--json out.json] [--repo PATH]

2) 의 마지막 줄은 `ATTR_LOSS_TOTAL=<n>` 이다. 표본을 찾지 못했거나 프로브가 실패한 사례가 있으면
숫자 대신 `UNMEASURED` 를 내고 0 이 아닌 코드로 끝난다(측정 못 한 것을 0 으로 보고하지 않는다).

손실로 세는 키(COUNTED)는 v353 게이트 G2 의 범위다 — 표 `lineWrap·noAdjust·lock·protect·
pos.*·sz.*·textFlow·pageBreak` 와 표 구조(표 수·셀 집합·캡션·cellzone). 그 밖의 차이(INFO)는
따로 보고만 하고 합계에 넣지 않는다. 그림(`hp:pic`)은 Phase 4 소관이라 INFO 다.
"""

import argparse
import datetime
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import zipfile
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
PIPELINE = "rhwp::DocumentCore::from_bytes -> export_hwpx_native"

COUNTED = {
    "tbl": ("textWrap", "textFlow", "lock", "pageBreak", "repeatHeader", "noAdjust",
            "cellSpacing", "rowCnt", "colCnt"),
    "sz": ("width", "widthRelTo", "height", "heightRelTo", "protect"),
    "pos": ("treatAsChar", "affectLSpacing", "flowWithText", "allowOverlap", "holdAnchorAndSO",
            "vertRelTo", "horzRelTo", "vertAlign", "horzAlign", "vertOffset", "horzOffset"),
    "caption": ("side", "fullSz", "width", "gap", "lastWidth", "@paragraphs"),
    "cellzone": ("startRowAddr", "startColAddr", "endRowAddr", "endColAddr", "borderFillIDRef"),
    "tc": ("protect", "editable", "dirty"),
    "subList": ("lineWrap", "vertAlign"),
}
INFO = {
    "tbl": ("id", "zOrder", "numberingType", "dropcapstyle", "borderFillIDRef"),
    "tc": ("name", "header", "hasMargin", "borderFillIDRef"),
    "subList": ("textDirection",),
}
EXAMPLES_PER_KEY = 3

PROBE_MAIN = r'''//! tools/roundtrip_diff.py 가 생성하는 왕복 프로브: HWPX 를 에디터 경로로 열고 다시 저장한다.
use std::{env, fs, process};

fn roundtrip(src: &str, dst: &str) -> Result<(), String> {
    let bytes = fs::read(src).map_err(|e| e.to_string())?;
    let core = rhwp::DocumentCore::from_bytes(&bytes).map_err(|e| e.to_string())?;
    let out = core.export_hwpx_native().map_err(|e| e.to_string())?;
    fs::write(dst, out).map_err(|e| e.to_string())
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut failed = 0;
    for pair in args.chunks(2) {
        if pair.len() != 2 {
            eprintln!("usage: probe <src> <dst> [<src> <dst> ...]");
            process::exit(64);
        }
        match roundtrip(&pair[0], &pair[1]) {
            Ok(()) => println!("OK\t{}", pair[0]),
            Err(e) => {
                failed += 1;
                println!("ERR\t{}\t{}", pair[0], e.replace('\n', " "));
            }
        }
    }
    if failed > 0 {
        process::exit(2);
    }
}
'''


def local(tag):
    return tag.rsplit("}", 1)[-1]


def child(el, name):
    if el is None:
        return None
    for c in el:
        if local(c.tag) == name:
            return c
    return None


def children(el, name):
    if el is None:
        return []
    return [c for c in el if local(c.tag) == name]


def section_names(z):
    names = [n for n in z.namelist() if re.fullmatch(r"Contents/section\d+\.xml", n)]
    return sorted(names, key=lambda n: int(re.search(r"(\d+)\.xml$", n).group(1)))


def table_record(tbl):
    rec = {"tbl": dict(tbl.attrib)}
    for name in ("sz", "pos"):
        c = child(tbl, name)
        rec[name] = dict(c.attrib) if c is not None else {}
    cap = child(tbl, "caption")
    if cap is not None:
        rec["caption"] = dict(cap.attrib)
        rec["caption"]["@paragraphs"] = str(len(children(child(cap, "subList"), "p")))
    else:
        rec["caption"] = None
    rec["cellzones"] = [dict(z.attrib) for z in children(child(tbl, "cellzoneList"), "cellzone")]
    cells = {}
    for tr in children(tbl, "tr"):
        for tc in children(tr, "tc"):
            addr = child(tc, "cellAddr")
            if addr is not None:
                key = (int(addr.get("rowAddr", "-1")), int(addr.get("colAddr", "-1")))
            else:
                key = ("noaddr", len(cells))
            sub = child(tc, "subList")
            cells[key] = {"tc": dict(tc.attrib), "subList": dict(sub.attrib) if sub is not None else {}}
    rec["cells"] = cells
    return rec


def pic_record(pic):
    org = child(pic, "orgSz")
    cur = child(pic, "curSz")
    return {"org": dict(org.attrib) if org is not None else {},
            "cur": dict(cur.attrib) if cur is not None else {}}


def extract(path):
    tables, pics = [], []
    with zipfile.ZipFile(path) as z:
        names = section_names(z)
        for name in names:
            root = ET.fromstring(z.read(name))
            for el in root.iter():
                ln = local(el.tag)
                if ln == "tbl":
                    tables.append(table_record(el))
                elif ln == "pic":
                    pics.append(pic_record(el))
    return tables, pics, names


class Tally:
    def __init__(self):
        self.loss = Counter()
        self.info = Counter()
        self.examples = []

    def add(self, bucket, key, where, before, after):
        counter = self.loss if bucket == "loss" else self.info
        counter[key] += 1
        if counter[key] <= EXAMPLES_PER_KEY:
            self.examples.append(f"{bucket}:{key} {where}: {before!r} -> {after!r}")


def compare_attrs(tally, label, where, a, b):
    for bucket, keys in (("loss", COUNTED.get(label, ())), ("info", INFO.get(label, ()))):
        for k in keys:
            va, vb = a.get(k), b.get(k)
            if va != vb:
                tally.add(bucket, f"{label}.{k}", where, va, vb)


def compare_files(src, dst):
    at, ap, _ = extract(src)
    bt, bp, _ = extract(dst)
    tally = Tally()
    if len(at) != len(bt):
        tally.loss["struct.tables"] += abs(len(at) - len(bt))
        tally.examples.append(f"loss:struct.tables {len(at)} -> {len(bt)}")
    for i, (ta, tb) in enumerate(zip(at, bt)):
        where = f"tbl[{i}]"
        for label in ("tbl", "sz", "pos"):
            compare_attrs(tally, label, where, ta[label], tb[label])
        if (ta["caption"] is None) != (tb["caption"] is None):
            tally.add("loss", "struct.caption", where, ta["caption"] is not None, tb["caption"] is not None)
        elif ta["caption"] is not None:
            compare_attrs(tally, "caption", where, ta["caption"], tb["caption"])
        if len(ta["cellzones"]) != len(tb["cellzones"]):
            tally.add("loss", "struct.cellzone", where, len(ta["cellzones"]), len(tb["cellzones"]))
        for zi, (za, zb) in enumerate(zip(ta["cellzones"], tb["cellzones"])):
            compare_attrs(tally, "cellzone", f"{where}.zone[{zi}]", za, zb)
        missing = set(ta["cells"]) ^ set(tb["cells"])
        if missing:
            tally.loss["struct.cells"] += len(missing)
            tally.examples.append(f"loss:struct.cells {where}: {len(missing)} cell address(es) differ")
        for key in sorted(set(ta["cells"]) & set(tb["cells"]), key=str):
            ca, cb = ta["cells"][key], tb["cells"][key]
            cw = f"{where}.cell{key}"
            compare_attrs(tally, "tc", cw, ca["tc"], cb["tc"])
            compare_attrs(tally, "subList", cw, ca["subList"], cb["subList"])
    if len(ap) != len(bp):
        tally.info["pic.count"] += abs(len(ap) - len(bp))
    for i, (pa, pb) in enumerate(zip(ap, bp)):
        for part in ("org", "cur"):
            for k in ("width", "height"):
                if pa[part].get(k) != pb[part].get(k):
                    tally.add("info", f"pic.{part}Sz.{k}", f"pic[{i}]", pa[part].get(k), pb[part].get(k))
    cells_a = sum(len(t["cells"]) for t in at)
    cells_b = sum(len(t["cells"]) for t in bt)
    return {
        "src": str(src),
        "dst": str(dst),
        "tables": [len(at), len(bt)],
        "cells": [cells_a, cells_b],
        "pics": [len(ap), len(bp)],
        "ATTR_LOSS_TOTAL": sum(tally.loss.values()),
        "INFO_DIFF_TOTAL": sum(tally.info.values()),
        "losses": dict(sorted(tally.loss.items())),
        "info": dict(sorted(tally.info.items())),
        "examples": tally.examples,
    }


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def git(repo, *args):
    try:
        out = subprocess.run(["git", "-C", str(repo), *args], capture_output=True, text=True, check=True)
        return out.stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return None


def build_probe(repo, work):
    probe = work / "probe"
    (probe / "src").mkdir(parents=True, exist_ok=True)
    manifest = (
        "[package]\n"
        'name = "rhwp-roundtrip-probe"\n'
        'version = "0.0.0"\n'
        'edition = "2021"\n'
        "publish = false\n\n"
        "[dependencies]\n"
        f"rhwp = {{ path = {json.dumps(str(repo))} }}\n\n"
        "[workspace]\n"
    )
    (probe / "Cargo.toml").write_text(manifest, encoding="utf-8")
    (probe / "src" / "main.rs").write_text(PROBE_MAIN, encoding="utf-8")
    if (repo / "Cargo.lock").is_file():
        shutil.copyfile(repo / "Cargo.lock", probe / "Cargo.lock")
    env = dict(os.environ)
    env["CARGO_TARGET_DIR"] = str(repo / "target")
    subprocess.run(
        ["cargo", "build", "--release", "--quiet", "--manifest-path", str(probe / "Cargo.toml")],
        env=env, check=True,
    )
    exe = repo / "target" / "release" / ("rhwp-roundtrip-probe" + (".exe" if os.name == "nt" else ""))
    if not exe.is_file():
        raise SystemExit(f"probe binary missing after build: {exe}")
    return exe


def find_case(fixtures, case):
    hits = sorted(fixtures.glob(f"{case}_*.hwpx"))
    return hits[0] if len(hits) == 1 else None


def print_case(case, res, summary):
    print(f"CASE {case} file={Path(res['src']).name} probe={res['probe']} "
          f"tables={res['tables'][0]}->{res['tables'][1]} cells={res['cells'][0]}->{res['cells'][1]} "
          f"pics={res['pics'][0]}->{res['pics'][1]} loss={res['ATTR_LOSS_TOTAL']} info={res['INFO_DIFF_TOTAL']}")
    for k, v in res["losses"].items():
        print(f"  loss {k}={v}")
    for k, v in res["info"].items():
        print(f"  info {k}={v}")
    if not summary:
        for e in res["examples"]:
            print(f"    {e}")


def run_fixtures(args):
    fixtures = Path(args.fixtures)
    repo = Path(args.repo).resolve() if args.repo else REPO_ROOT
    cases = [c.strip() for c in args.cases.split(",") if c.strip()]
    out_dir = Path(args.out_dir) if args.out_dir else REPO_ROOT / "target" / "table-fidelity-roundtrip"
    out_dir.mkdir(parents=True, exist_ok=True)

    report = {
        "tool": "tools/roundtrip_diff.py",
        "pipeline": PIPELINE,
        "repo": str(repo),
        "head": git(repo, "rev-parse", "HEAD"),
        "src_dirty": bool(git(repo, "status", "--porcelain", "--", "src", "Cargo.toml")),
        "generated_at": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "fixtures": str(fixtures),
        "cases": cases,
        "counted_keys": {k: list(v) for k, v in COUNTED.items()},
        "info_keys": {k: list(v) for k, v in INFO.items()},
    }

    unmeasured = []
    pairs = []
    for case in cases:
        src = find_case(fixtures, case)
        if src is None:
            unmeasured.append(case)
            report[case] = {"probe": "FIXTURE_NOT_FOUND"}
            print(f"CASE {case} FIXTURE_NOT_FOUND in {fixtures}")
            continue
        pairs.append((case, src, out_dir / f"{case}.roundtrip.hwpx"))

    probe_status = {}
    if pairs:
        exe = build_probe(repo, out_dir)
        for _, _, dst in pairs:
            if dst.exists():
                dst.unlink()
        argv = [str(exe)]
        for _, src, dst in pairs:
            argv += [str(src), str(dst)]
        run = subprocess.run(argv, capture_output=True, text=True)
        for line in run.stdout.splitlines():
            parts = line.split("\t")
            if len(parts) >= 2:
                probe_status[parts[1]] = "OK" if parts[0] == "OK" else "ERR " + " ".join(parts[2:])
        if run.stderr.strip():
            sys.stderr.write(run.stderr)

    total = 0
    for case, src, dst in pairs:
        status = probe_status.get(str(src), "NO_STATUS")
        if status != "OK" or not dst.is_file():
            unmeasured.append(case)
            report[case] = {"file": src.name, "sha256": sha256(src), "probe": status}
            print(f"CASE {case} file={src.name} probe={status}")
            continue
        res = compare_files(src, dst)
        res["probe"] = status
        res["file"] = src.name
        res["sha256"] = sha256(src)
        report[case] = res
        total += res["ATTR_LOSS_TOTAL"]
        print_case(case, res, args.summary)

    report["unmeasured"] = unmeasured
    if unmeasured:
        report["ATTR_LOSS_TOTAL"] = None
        print(f"ATTR_LOSS_TOTAL=UNMEASURED({','.join(unmeasured)})")
    else:
        report["ATTR_LOSS_TOTAL"] = total
        print(f"ATTR_LOSS_TOTAL={total}")
    if args.json:
        Path(args.json).parent.mkdir(parents=True, exist_ok=True)
        with open(args.json, "w", encoding="utf-8") as f:
            json.dump(report, f, ensure_ascii=False, indent=2)
            f.write("\n")
    return 1 if unmeasured else 0


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("pair", nargs="*", help="SRC.hwpx DST.hwpx (두 파일 비교)")
    ap.add_argument("--fixtures", help="표본 폴더 (T<n>_*.hwpx)")
    ap.add_argument("--cases", default="T1,T2,T6,T7", help="쉼표로 구분한 사례 (기본 T1,T2,T6,T7)")
    ap.add_argument("--repo", help="프로브가 링크할 rhwp 저장소 (기본: 이 도구의 저장소)")
    ap.add_argument("--out-dir", help="왕복본 출력 폴더 (기본: target/table-fidelity-roundtrip)")
    ap.add_argument("--json", help="결과 JSON 경로")
    ap.add_argument("--summary", action="store_true", help="예시 줄 없이 집계만 출력")
    args = ap.parse_args(argv)

    if args.fixtures:
        return run_fixtures(args)
    if len(args.pair) != 2:
        ap.error("SRC DST 두 파일 또는 --fixtures 가 필요하다")
    res = compare_files(Path(args.pair[0]), Path(args.pair[1]))
    res["probe"] = "n/a"
    print_case("pair", res, args.summary)
    print(f"ATTR_LOSS_TOTAL={res['ATTR_LOSS_TOTAL']}")
    if args.json:
        with open(args.json, "w", encoding="utf-8") as f:
            json.dump(res, f, ensure_ascii=False, indent=2)
            f.write("\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
