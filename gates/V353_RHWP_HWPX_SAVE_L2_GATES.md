# Gates: v353 rhwp HWPX save fidelity lane 2

- [x] G1: 변경 전 baseline — 59c2285 에서 G8 시험 실패, 표 앞 run 순서 결함(인라인 표 앞에 공백 run) 을 기록
  CHECK: python3 -c 'import json;d=json.load(open(".omo/evidence/v353-rhwp-hwpx-save-l2/baseline.json"));assert d["head"]=="59c2285efe616211cd147dfa849d2f9f9df767dd";assert d["g8_failed"] is True and d["order_defects"]>0;print("BASELINE_OK",d["order_defects"])'
  EXPECT: /^BASELINE_OK \d+$/m
  EVIDENCE: automatic-evidence=v1; definition-sha256=3fa1521b5b252efd4fbb5c462a7fb0524f3a3b2290e2eee30356d7366cccf763; exit=0; EXPECT=matched; output-sha256=b7c7b01c836299fdb4f6bc06288e883a1bda506de161f68b2205c051bf932fb8; output-bytes=15; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=802bd5fa384c/47 entries

- [x] G2: 문단 run/컨트롤 순서 — 원본과 재저장본의 각 문단 run·컨트롤 순서열이 같다(본문·셀·중첩 표, fixture 전부)
  CHECK: cargo test --test hwpx_save_fidelity_l2 run_order 2>&1 | tail -3
  EXPECT: /test result: ok\. [1-9]\d* passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=1f8be6726c361c60fdf2f23c680c73a02f790249b8ba975b37b4766c5f4ec0ac; exit=0; EXPECT=matched; output-sha256=8546936eb01e615cbaf0b01b7614bd115e04c13d4b003b4eb9c83f958e734a94; output-bytes=97; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=43712a571703/49 entries

- [x] G3: 두 번 저장 — secPr/colPr/pageNum run 구성 유지, 기존 G8 시험 통과
  CHECK: cargo test --lib serializer::hwpx::tests::form_attribute_entities_survive_two_hwpx_saves 2>&1 | tail -3
  EXPECT: /test result: ok\. 1 passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=f4b9052439b84f4a4bceaf9768a1c8660ee84880fd71edf0d7e5d45eb3f3073e; exit=0; EXPECT=matched; output-sha256=8dfc7d202aad63fd82d5c261b55e83fbf6922b238cc20a271f7c12e199e83a2a; output-bytes=99; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=43712a571703/49 entries

- [x] G4: 도형·필드·header 잔여 결함 측정과 수정 — 새 base 에서 측정한 결함 목록과, 그 각각의 회귀 시험
  CHECK: cargo test --test hwpx_save_fidelity_l2 residual 2>&1 | tail -3
  EXPECT: /test result: ok\. [1-9]\d* passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=d6d98cda6ebeffa990fe4459455f282f349524f9cee3f3f7e7a0b7ea3e591715; exit=0; EXPECT=matched; output-sha256=9b6f1dd8c2396068c5ed649470cf14147b1cad958c0b691f00194c4518f83bd2; output-bytes=98; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=43712a571703/49 entries

- [x] G5: 음성 대조 — G2·G3·G4 시험이 59c2285 에서는 실패한다
  CHECK: python3 -c 'import json,hashlib,pathlib,re;d=json.load(open(".omo/evidence/v353-rhwp-hwpx-save-l2/negative-control.json"));assert d["base"]=="59c2285efe616211cd147dfa849d2f9f9df767dd";assert d["run_order_failed"]>0 and d["two_save_failed"]>0 and d["residual_failed"]>0;assert d["test_file_sha256"]==hashlib.sha256(pathlib.Path("tests/hwpx_save_fidelity_l2.rs").read_bytes()).hexdigest();s=pathlib.Path(".omo/evidence/v353-rhwp-hwpx-save-l2/c1/red-8027a28-final.log").read_text();assert re.search(r"test result: FAILED\. 1 passed; 5 failed",s);print("NEG_OK")'
  EXPECT: /^NEG_OK$/m
  EVIDENCE: automatic-evidence=v1; definition-sha256=191200e74da1f39ebe0ffc4ffc263ae0454c5f7ecb664655b0e0c4656310570a; exit=0; EXPECT=matched; output-sha256=b5568d8ad4ed8d6a9fab8348e510711f8358c218705c2d8f502ebf2bbb470a0f; output-bytes=7; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=43712a571703/49 entries

- [x] G6: 레인 1 회귀 없음 — 표 충실도 시험 전부
  CHECK: cargo test --test hwpx_table_fidelity 2>&1 | tail -3
  EXPECT: /test result: ok\. [1-9]\d* passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=9a2a0ae422a6e8a93e9228879468f37b3f9bb2dc2b7af3f04efc72afb4818b5d; exit=0; EXPECT=matched; output-sha256=8080cd2a72ca687efd45532dfe8f5bfcc9a191231d7f91ee4a06a52bf8e9a20d; output-bytes=97; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=43712a571703/49 entries

- [x] G7: 전체 시험
  CHECK: bash .omo/evidence/v353-rhwp-hwpx-save-l2/c1/clean-cargo.sh test --no-fail-fast > .omo/evidence/v353-rhwp-hwpx-save-l2/c1/g7-full.log 2>&1 && python3 -c 'import pathlib,re;s=pathlib.Path(".omo/evidence/v353-rhwp-hwpx-save-l2/c1/g7-full.log").read_text();r=re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored",s);assert r and all(int(x[1])==0 for x in r);print("FAILED_TOTAL=0")'
  EXPECT: /^FAILED_TOTAL=0$/m
  EVIDENCE: automatic-evidence=v1; definition-sha256=68973d8e72bba97554bf2d826267e99ed285ef0d3dd007b6e2fa9c2f90e4c5de; exit=0; EXPECT=matched; output-sha256=34acd69d8c8337c7c993bc0a99093bb0f4dfc879c5cea9515b73e5957c9ccf55; output-bytes=15; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=43712a571703/49 entries

- [ ] G8: clippy
  CHECK: bash .omo/evidence/v353-rhwp-hwpx-save-l2/c1/clean-cargo.sh clippy -- -D warnings > .omo/evidence/v353-rhwp-hwpx-save-l2/c1/g8-clippy.log 2>&1 && tail -1 .omo/evidence/v353-rhwp-hwpx-save-l2/c1/g8-clippy.log
  EXPECT: /Finished/
  EVIDENCE: pending

- [x] G9: WASM 빌드
  CHECK: bash .omo/evidence/v353-rhwp-hwpx-save-l2/c1/clean-cargo.sh build --lib --target wasm32-unknown-unknown --release > .omo/evidence/v353-rhwp-hwpx-save-l2/c1/g9-wasm.log 2>&1 && tail -1 .omo/evidence/v353-rhwp-hwpx-save-l2/c1/g9-wasm.log
  EXPECT: /Finished/
  EVIDENCE: automatic-evidence=v1; definition-sha256=bf02372c67b1959fdbf6a4efb651b8e8ba3440a9d25c2a2c6972ddbdc023a91b; exit=0; EXPECT=matched; output-sha256=7bf5ae2b3f792e49eb1b5d0f51ebce57ad07c3204f04aecc9bb4a502986e02a2; output-bytes=63; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=43712a571703/49 entries

- [x] G10: 범위 — 59c2285 대비 변경 파일이 §3.1 안
  CHECK: git diff --name-only 59c2285efe616211cd147dfa849d2f9f9df767dd HEAD | grep -vE '^(src/serializer/hwpx/(section|table|header|shape|field|mod)\.rs|src/parser/hwpx/(header|section)\.rs|src/model/document\.rs|tests/hwpx_save_fidelity_l2\.rs|tests/hwpx_roundtrip_integration\.rs|tests/fixtures/save-fidelity-l2/.*|gates/V353_RHWP_HWPX_SAVE_L2_GATES\.md|\.omo/evidence/v353-rhwp-hwpx-save-l2/.*)$' | wc -l | tr -d ' ' | sed 's/^/OUT_OF_SCOPE=/'
  EXPECT: /^OUT_OF_SCOPE=0$/m
  EVIDENCE: automatic-evidence=v1; definition-sha256=44feb6533ebefeda55f4f76bd49abe56b16561cde5e6204ae77e6e4853787e00; exit=0; EXPECT=matched; output-sha256=13a10730bd1c382bb5fe6ac3f0e5a982453970f00ca15e1b5fda9516047c9824; output-bytes=15; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=43712a571703/49 entries

- [x] G11: 한컴 확인용 산출물 — fixture 2개 × (열고 저장 / 행 삽입 / 셀 글자 입력) 을 .omo/evidence/v353-rhwp-hwpx-save-l2/hancom/ 에 저장(한컴 열기·시각 확인은 Parent)
  CHECK: ls .omo/evidence/v353-rhwp-hwpx-save-l2/hancom/*.hwpx | wc -l | tr -d ' ' | sed 's/^/HANCOM_FILES=/'
  EXPECT: /^HANCOM_FILES=([6-9]|[1-9][0-9])$/m
  EVIDENCE: automatic-evidence=v1; definition-sha256=135f77ee1dc8eb0787e171e9d12461a00a9d93c710125c4a8b85fb49f5f3973e; exit=0; EXPECT=matched; output-sha256=62f24220e0b5abf0f9e344091f298c6d33b749ca253b1b996878ce867b588c7d; output-bytes=15; shell=/bin/sh; cwd=/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity; path=43712a571703/49 entries

## NOTE

- NOTE G2 (Parent 처분 1, `outputs/v353-rhwp-table/RHWP_HWPX_SAVE_FIDELITY_L2_PACKET_DISPOSITION1.md`): 이 포크의 HWPX 파서는 필드가 아닌 컨트롤을 모두 char_offsets 앞머리에 모은다(`src/parser/hwpx/section.rs` ctrl_offset). 그래서 C6 최상위 p195(공백 5개 뒤 글자처럼 취급하는 표)는 IR 순서인 «표 → 공백» 으로 저장된다(fixture 마다 2050 문단 중 1). `run_order_*` 시험은 그 한 문단만 IR 순서를 기대하고, 그런 원본 문단이 정확히 그 하나뿐임을 함께 단언한다. 글자 위치 정밀 보존(파서 ctrl_offset)은 별도 후속 레인 후보다.
- NOTE G2 기준: 두 fixture 모두 한컴 원본 C6 의 문단 순서열과 비교한다. FACF 는 C6 를 G0 시기 rhwp 가 내보낸 파일이라 문단·글자가 C6 와 같고(시험이 단언), FACF 자신의 run 순서는 이 레인이 고치는 결함 출력이다. 59c2285 에서 두 열고 저장본 모두 C6 대비 55 문단(셀 colPr 38 · rect 10 · 공백 run 뒤 인라인 표 7)이 어긋났다(`baseline.json`).
- NOTE G4 측정(59c2285, C6·FACF 열고 저장 대비 C6, `hp:line` 은 T6): 고친 것 — (1) header paraPr 여백·줄 간격을 `hp:switch` 없이 default(2배) 값 평문으로 씀 → HwpUnitChar case(절반, 홀수는 unit=CHAR)/default switch, `xmlns:hwpunitchar` 선언 (2) tabItem 도 같은 2배 평문 → tabItem 마다 switch (3) 묶음 안 자식 도형에 sz/pos/outMargin 을 씀(원본 10개는 없음) → 쓰지 않음 (4) 음수 `hp:offset` 을 부호 있는 표기로 써서 다시 열면 0(묶음 자식 4개, 두 번째 저장에서 배지 위치 이동) → 한컴처럼 32비트 부호 없는 표기 (5) `hp:line` 에 OWPML 필수 자식 `hc:startPt`/`hc:endPt` 가 없음 → 씀(`startX`… 속성은 이 포크 파서가 읽어 남김) (6) Parent 처분 2 — 홀수 여백 case unit=CHAR 를 파서가 무시해 1 줄어듦 → 파서가 +1. 필드는 59c2285 에서 fieldBegin/End 11/11 바이트 동일·원시 제어문자 0 이라 고칠 것이 없었다.
- NOTE G4 남긴 것(§3.1 밖 파서·모델이 값을 담지 않음): 도형 `id`(파서는 instid 만 읽음)·`numberingType`(PICTURE→NONE)·도형 pos `allowOverlap`/`flowWithText`·`rotateimage`·lineShape 화살표(headSz/tailSz/tailStyle)·renderingInfo 행렬 분해(IR 은 합성 행렬 하나), 표 `id`/`zOrder`(0 으로 씀), FACF 의 `-395` offset 4개(파서 parse_u32 가 0 으로 읽어 FACF 열고 저장에서 이미 잃음), 한컴 선 좌표(파서가 startPt/endPt 를 읽지 않아 0), T6 의 polygon 묶음·수식(직렬화기 미지원으로 빠짐), 묶음 안 그림(picture.rs 는 §3.1 밖).
- NOTE G7: 이 CHECK 는 awk 로 끝나므로 어떤 시험 묶음이 컴파일에 실패하면 그 묶음의 `test result:` 줄이 없어 FAILED_TOTAL=0 으로 읽힌다. 패킷 CHECK 를 그대로 두고, 같은 커밋에서 `bash -o pipefail` 로 cargo 종료 코드와 `test result:` 줄 수·통과 합을 따로 잰다(아래 결과 줄).
- NOTE G10 (Parent 처분 2, `outputs/v353-rhwp-table/RHWP_HWPX_SAVE_FIDELITY_L2_PACKET_DISPOSITION2.md`): 허용 목록에 `tests/hwpx_roundtrip_integration.rs`(c10 세 시험·c9 여백 시험의 기대값만)와 `src/parser/hwpx/header.rs`(switch case unit=CHAR 홀수 처리만)를 더했다. 근거 대조 — 한컴 12.30 MAC64 저장본 5개의 case unit=CHAR 값 43개가 모두 default = case×2+1(예외 0, `char-unit-evidence.json`).
- 결과 G7 독립 측정(source HEAD `c3059ef6cf372bb201bf4a36e36fc9092439db9b`): `bash -o pipefail` cargo_exit=0 · 시험 바이너리 17개 시작 · `test result:` 17줄 · 1246 passed / 0 failed / 7 ignored(`.omo/evidence/v353-rhwp-hwpx-save-l2/gate-raw-output.txt`).
- 결과: source HEAD `c3059ef6cf372bb201bf4a36e36fc9092439db9b` 에서 unlazy `gate-check.mjs --approve --cwd <worktree> --timeout 3600` → `ALL MET (11 met)`(`gate-check-output.txt`). 수정 뒤 측정 `after.json`: C6·FACF 열고 저장본 모두 C6 대비 순서가 다른 문단 1개(p195, Parent 처분 1)·인라인 표 앞 공백 run 0·header switch 463(원본과 같음)·묶음 자식 구성 차이 0·T6 선 자식 구성 원본과 같음.


## C1 correction delta (source c0d6a342c186a3d57c416ddb262783cc7a064c23)

- NOTE G1 carry: baseline.json and its CHECK inputs are byte-identical to 8027a28e4d1429f11cec0860d11484d1d9e6e3fe; 8027a28..5315315 changes only the six source/test paths listed below. The original c3059ef G1 evidence remains applicable; it is not resealed as a new baseline.
- NOTE G10 scope addition (Parent C1 brief): src/parser/hwpx/header.rs may read trackchageConfig@flags and correct F2/F3 read-back; src/model/document.rs adds only hwpx_track_change_flags (u32, default 0). Parent answer msg_f68ad7c08797 / outputs/v353-rhwp-table/RHWP_HWPX_SAVE_L2_CORRECTION_C1_V353_DISPOSITION1.md additionally permits src/parser/hwpx/section.rs for begin_order/end_order calculations only. Both parser paths and the model path are included in CHECK. No other model field, renderer, table geometry, pageBreak, or V318 ledger change.
- NOTE G2 F1: fields and ordinary controls are merged into one ordered event stream in table.rs, shared by body section.rs and cell/nested paragraphs. HWPX field order counts control starts plus completed field ends; char_offsets placement is unchanged. New raw-XML cases cover [begin,end,tbl], [begin,tbl,end], [tbl,begin,end], nested and adjacent empty fields, in body/cell/nested-cell owners, over two native saves; the reviewer's binary-field IR case is retained as a separate test.
- NOTE G4 F2/F3 halving-path inventory: TabItem.position; ParaShape.indent (hc:intent), margin_left (hc:left), margin_right (hc:right), spacing_before (hc:prev), spacing_after (hc:next); ParaShape.line_spacing for Fixed, SpaceOnly/BETWEEN_LINES, Minimum/AT_LEAST. Percent is unscaled. Odd case values use floor(value/2) with unit=CHAR, even values HWPUNIT; default values retain the full original. The reader restores case*2+CHAR. SpaceOnly's emitted BETWEEN_LINES spelling is recognized in the plain and switch read paths.
- NOTE G4 signed coverage: all five margins and all three scaled line-spacing types test -1001/-1000/-1/0/1/1000/1001/i32::MIN/i32::MIN+1/i32::MAX-1/i32::MAX across two saves; Percent is checked unchanged over the same values. Tab positions use 0/1/1000/1001/2^31/2^31+1/u32::MAX-1/u32::MAX. Parent answer to the second C1 ask (persisted in Orca) approves treating negative tabs as unrepresentable in the u32 model; no signed-tab model expansion is made.
- NOTE G4 F4: C6's original flags=56, explicit 0 and u32::MAX survive two native saves for both empty and paired element spellings. An absent element is compared with explicit-zero output at each save and retains today's behavior. The model carries raw flags without assigning meanings to its bits.
- NOTE G5 C1: latest test SHA256=31b58bbaf1a8d7a1426c192aa1113190d801c350124dc4ac89249aef517bd066. Isolated 8027a28 archive: 1 passed / 5 failed (the absent-element control passes). Updated 59c2285 negative-control.json: L2 2 passed / 16 failed / 2 ignored, run_order_failed=5, two_save_failed=3 (including G3), residual_failed=9. This supplements the original base evidence rather than replacing its meaning.
- NOTE G7/G8/G9: clean-cargo.sh executes the declared cargo command from git archive HEAD plus a byte-identical copy of the untracked Cargo.lock, using the existing target cache. Cargo exit codes are propagated; a compilation failure can no longer turn into FAILED_TOTAL=0 through an empty awk input. Each log records the exact source HEAD and command. These are local checks; no CI or Hancom UI result is claimed.
- NOTE dependency delta: c1/impact-0.log through impact-5.log record mah impact --code-ref HEAD for every changed source/test file at 8027a28 (Jev used). All deterministic pin/exec results were retained. Related lane-1/G0 executable coverage is supplied by G6/G7 and the direct green-g0.log (57 passed / 2 ignored); their historical ledgers are untouched.
- NOTE assurance: current-session diff inspection and actual scenarios are executor checks only; no self-review receipt was created. Separate Codex re-review and Parent Hancom visual confirmation remain externally dispatched stages.

- NOTE C1 lint follow-up: initial clean-source run at 5315315 passed G2-G7 and G9-G11, but G8 failed with manual_is_multiple_of on the new unsigned tab parity expression. Source c0d6a34 replaces only that expression with is_multiple_of(2); initial raw outputs are retained as *-initial-5315315.log. Per Parent STOP-SAVE msg_450f6ddd4d0d, the obtained G2-G7/G9-G11 evidence at 5315315 is retained and G8 remains pending. Source c0d6a34 has only the parity lint edit and has not been revalidated. G1 remains the unchanged baseline carry.

- NOTE PAUSED (Parent STOP-SAVE, 2026-10-26T08:10:19Z): G7 obtained 1252 passed / 0 failed / 7 ignored in 17 groups and G9 obtained Finished at exact source 5315315566c6fe75826fe05a9d59880aae62948a. G8's one lint is fixed in c0d6a342c186a3d57c416ddb262783cc7a064c23 but has not been rerun; its box and EVIDENCE remain pending. The checked rows record the already obtained source-5315315 evidence, not a claim of final-head validation; final correction validation/re-review and final-head Hancom regeneration remain for resumption. No new cargo/clippy/WASM was started after STOP-SAVE. No worker_done or successful mah finish is issued while this task is paused.
