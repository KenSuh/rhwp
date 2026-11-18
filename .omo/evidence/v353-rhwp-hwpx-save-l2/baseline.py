# 수정 전(59c2285) 트리에서 G8 시험 실패와 문단 run 순서·잔여 결함을 재 baseline.json 에 남긴다.
# 사용: python3 -I baseline.py <저장소> <출력 json>  (저장소 src/ 가 59c2285 와 같아야 한다)
import datetime, json, os, re, subprocess, sys, tempfile, zipfile

repo, out_json = sys.argv[1], sys.argv[2]
here = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, here)
import l2_measure as m  # noqa: E402

BASE = "59c2285efe616211cd147dfa849d2f9f9df767dd"


def git(*args):
    return subprocess.run(["git", "-C", repo, *args], capture_output=True, text=True, check=True).stdout


head = git("rev-parse", "HEAD").strip()
src_dirty = git("status", "--porcelain", "--", "src").strip() != ""
src_diff_vs_base = git("diff", "--name-only", BASE, "--", "src").strip()

g8 = subprocess.run(
    ["cargo", "test", "--lib", "serializer::hwpx::tests::form_attribute_entities_survive_two_hwpx_saves"],
    cwd=repo, capture_output=True, text=True)
g8_text = g8.stdout + g8.stderr
g8_result = re.search(r"test result: (\w+)\. (\d+) passed; (\d+) failed", g8_text)
g8_panic = re.findall(r"panicked at [^\n]*\n([^\n]*)", g8_text)

dump = tempfile.mkdtemp(prefix="l2-baseline-")
d = subprocess.run(
    ["cargo", "test", "--test", "hwpx_save_fidelity_l2", "dump_open_save", "--", "--ignored"],
    cwd=repo, capture_output=True, text=True, env={**os.environ, "L2_DUMP_OUT": dump})
assert d.returncode == 0, d.stdout + d.stderr

fx = os.path.join(repo, "tests/fixtures/save-fidelity-l2")
c6 = os.path.join(fx, "C6_hancom_original.hwpx")
facf = os.path.join(fx, "FACF_gearup_template.hwpx")
t6 = os.path.join(repo, "tests/fixtures/table-fidelity/T6_table_giant_cell_overfill.hwpx")

order = {}
for label, path in [("C6_open_save", f"{dump}/C6_open_save.hwpx"), ("FACF_open_save", f"{dump}/FACF_open_save.hwpx"),
                    ("FACF_template_itself", facf)]:
    r = m.compare(c6, path, label)
    order[label] = {k: r[k] for k in ("order_diff_paras", "ws_before_inline_tbl", "text_only_diff_paras", "order_diff_patterns")}


def line_children(path):
    x = zipfile.ZipFile(path).read("Contents/section0.xml").decode("utf-8")
    return [re.findall(r"<(?:hc|hp):(\w+)", blk)[1:] for blk in re.findall(r"<hp:line .*?</hp:line>", x, re.S)]


residual = {
    "header": {k: m.header_report(v) for k, v in [("C6_original", c6), ("C6_open_save", f"{dump}/C6_open_save.hwpx")]},
    "fields": {k: m.field_report(v) for k, v in [("C6_original", c6), ("C6_open_save", f"{dump}/C6_open_save.hwpx")]},
    "shapes_C6_open_save": m.shape_report(c6, f"{dump}/C6_open_save.hwpx"),
    "line_children": {"T6_original": line_children(t6), "T6_open_save": line_children(f"{dump}/T6_open_save.hwpx")},
}
for v in residual["header"].values():
    v["breakNonLatinWord"] = dict(v["breakNonLatinWord"])

res = {
    "head": head,
    "src_dirty": src_dirty,
    "src_diff_vs_base": src_diff_vs_base.splitlines(),
    "generated_at": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "g8_test": "serializer::hwpx::tests::form_attribute_entities_survive_two_hwpx_saves",
    "g8_failed": bool(g8_result and g8_result.group(1) == "FAILED" and int(g8_result.group(3)) == 1),
    "g8_result_line": g8_result.group(0) if g8_result else None,
    "g8_panic_first_line": g8_panic[:1],
    # 한컴 원본 C6 대비 C6 열고 저장본에서 run·컨트롤 순서가 다른 문단 수
    "order_defects": order["C6_open_save"]["order_diff_paras"],
    "order": order,
    "residual": residual,
}
json.dump(res, open(out_json, "w"), ensure_ascii=False, indent=1, default=str)
print("head", head, "src_dirty", src_dirty, "g8_failed", res["g8_failed"], "order_defects", res["order_defects"],
      "ws_before_inline_tbl", order["C6_open_save"]["ws_before_inline_tbl"])
