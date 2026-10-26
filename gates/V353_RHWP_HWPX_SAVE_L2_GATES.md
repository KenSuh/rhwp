# Gates: v353 rhwp HWPX save fidelity lane 2

- [ ] G1: 변경 전 baseline — 59c2285 에서 G8 시험 실패, 표 앞 run 순서 결함(인라인 표 앞에 공백 run) 을 기록
  CHECK: python3 -c 'import json;d=json.load(open(".omo/evidence/v353-rhwp-hwpx-save-l2/baseline.json"));assert d["head"]=="59c2285efe616211cd147dfa849d2f9f9df767dd";assert d["g8_failed"] is True and d["order_defects"]>0;print("BASELINE_OK",d["order_defects"])'
  EXPECT: /^BASELINE_OK \d+$/m
  EVIDENCE: pending

- [ ] G2: 문단 run/컨트롤 순서 — 원본과 재저장본의 각 문단 run·컨트롤 순서열이 같다(본문·셀·중첩 표, fixture 전부)
  CHECK: cargo test --test hwpx_save_fidelity_l2 run_order 2>&1 | tail -3
  EXPECT: /test result: ok\. [1-9]\d* passed; 0 failed/
  EVIDENCE: pending

- [ ] G3: 두 번 저장 — secPr/colPr/pageNum run 구성 유지, 기존 G8 시험 통과
  CHECK: cargo test --lib serializer::hwpx::tests::form_attribute_entities_survive_two_hwpx_saves 2>&1 | tail -3
  EXPECT: /test result: ok\. 1 passed; 0 failed/
  EVIDENCE: pending

- [ ] G4: 도형·필드·header 잔여 결함 측정과 수정 — 새 base 에서 측정한 결함 목록과, 그 각각의 회귀 시험
  CHECK: cargo test --test hwpx_save_fidelity_l2 residual 2>&1 | tail -3
  EXPECT: /test result: ok\. [1-9]\d* passed; 0 failed/
  EVIDENCE: pending

- [ ] G5: 음성 대조 — G2·G3·G4 시험이 59c2285 에서는 실패한다
  CHECK: python3 -c 'import json;d=json.load(open(".omo/evidence/v353-rhwp-hwpx-save-l2/negative-control.json"));assert d["base"]=="59c2285efe616211cd147dfa849d2f9f9df767dd";assert d["run_order_failed"]>0 and d["two_save_failed"]>0;print("NEG_OK")'
  EXPECT: /^NEG_OK$/m
  EVIDENCE: pending

- [ ] G6: 레인 1 회귀 없음 — 표 충실도 시험 전부
  CHECK: cargo test --test hwpx_table_fidelity 2>&1 | tail -3
  EXPECT: /test result: ok\. [1-9]\d* passed; 0 failed/
  EVIDENCE: pending

- [ ] G7: 전체 시험
  CHECK: cargo test --no-fail-fast 2>&1 | grep -E "^test result:" | awk '{f+=$6} END {print "FAILED_TOTAL=" f+0}'
  EXPECT: /^FAILED_TOTAL=0$/m
  EVIDENCE: pending

- [ ] G8: clippy
  CHECK: cargo clippy -- -D warnings 2>&1 | tail -1
  EXPECT: /Finished/
  EVIDENCE: pending

- [ ] G9: WASM 빌드
  CHECK: cargo build --lib --target wasm32-unknown-unknown --release 2>&1 | tail -1
  EXPECT: /Finished/
  EVIDENCE: pending

- [ ] G10: 범위 — 59c2285 대비 변경 파일이 §3.1 안
  CHECK: git diff --name-only 59c2285efe616211cd147dfa849d2f9f9df767dd HEAD | grep -vE '^(src/serializer/hwpx/(section|table|header|shape|field|mod)\.rs|src/parser/hwpx/header\.rs|tests/hwpx_save_fidelity_l2\.rs|tests/hwpx_roundtrip_integration\.rs|tests/fixtures/save-fidelity-l2/.*|gates/V353_RHWP_HWPX_SAVE_L2_GATES\.md|\.omo/evidence/v353-rhwp-hwpx-save-l2/.*)$' | wc -l | tr -d ' ' | sed 's/^/OUT_OF_SCOPE=/'
  EXPECT: /^OUT_OF_SCOPE=0$/m
  EVIDENCE: pending

- [ ] G11: 한컴 확인용 산출물 — fixture 2개 × (열고 저장 / 행 삽입 / 셀 글자 입력) 을 .omo/evidence/v353-rhwp-hwpx-save-l2/hancom/ 에 저장(한컴 열기·시각 확인은 Parent)
  CHECK: ls .omo/evidence/v353-rhwp-hwpx-save-l2/hancom/*.hwpx | wc -l | tr -d ' ' | sed 's/^/HANCOM_FILES=/'
  EXPECT: /^HANCOM_FILES=([6-9]|[1-9][0-9])$/m
  EVIDENCE: pending

## NOTE

- NOTE G2 (Parent 처분 1, `outputs/v353-rhwp-table/RHWP_HWPX_SAVE_FIDELITY_L2_PACKET_DISPOSITION1.md`): 이 포크의 HWPX 파서는 필드가 아닌 컨트롤을 모두 char_offsets 앞머리에 모은다(`src/parser/hwpx/section.rs` ctrl_offset). 그래서 C6 최상위 p195(공백 5개 뒤 글자처럼 취급하는 표)는 IR 순서인 «표 → 공백» 으로 저장된다(fixture 마다 2050 문단 중 1). `run_order_*` 시험은 그 한 문단만 IR 순서를 기대하고, 그런 원본 문단이 정확히 그 하나뿐임을 함께 단언한다. 글자 위치 정밀 보존(파서 ctrl_offset)은 별도 후속 레인 후보다.
- NOTE G2 기준: 두 fixture 모두 한컴 원본 C6 의 문단 순서열과 비교한다. FACF 는 C6 를 G0 시기 rhwp 가 내보낸 파일이라 문단·글자가 C6 와 같고(시험이 단언), FACF 자신의 run 순서는 이 레인이 고치는 결함 출력이다. 59c2285 에서 두 열고 저장본 모두 C6 대비 55 문단(셀 colPr 38 · rect 10 · 공백 run 뒤 인라인 표 7)이 어긋났다(`baseline.json`).
- NOTE G4 측정(59c2285, C6·FACF 열고 저장 대비 C6, `hp:line` 은 T6): 고친 것 — (1) header paraPr 여백·줄 간격을 `hp:switch` 없이 default(2배) 값 평문으로 씀 → HwpUnitChar case(절반, 홀수는 unit=CHAR)/default switch, `xmlns:hwpunitchar` 선언 (2) tabItem 도 같은 2배 평문 → tabItem 마다 switch (3) 묶음 안 자식 도형에 sz/pos/outMargin 을 씀(원본 10개는 없음) → 쓰지 않음 (4) 음수 `hp:offset` 을 부호 있는 표기로 써서 다시 열면 0(묶음 자식 4개, 두 번째 저장에서 배지 위치 이동) → 한컴처럼 32비트 부호 없는 표기 (5) `hp:line` 에 OWPML 필수 자식 `hc:startPt`/`hc:endPt` 가 없음 → 씀(`startX`… 속성은 이 포크 파서가 읽어 남김) (6) Parent 처분 2 — 홀수 여백 case unit=CHAR 를 파서가 무시해 1 줄어듦 → 파서가 +1. 필드는 59c2285 에서 fieldBegin/End 11/11 바이트 동일·원시 제어문자 0 이라 고칠 것이 없었다.
- NOTE G4 남긴 것(§3.1 밖 파서·모델이 값을 담지 않음): 도형 `id`(파서는 instid 만 읽음)·`numberingType`(PICTURE→NONE)·도형 pos `allowOverlap`/`flowWithText`·`rotateimage`·lineShape 화살표(headSz/tailSz/tailStyle)·renderingInfo 행렬 분해(IR 은 합성 행렬 하나), 표 `id`/`zOrder`(0 으로 씀), FACF 의 `-395` offset 4개(파서 parse_u32 가 0 으로 읽어 FACF 열고 저장에서 이미 잃음), 한컴 선 좌표(파서가 startPt/endPt 를 읽지 않아 0), T6 의 polygon 묶음·수식(직렬화기 미지원으로 빠짐), 묶음 안 그림(picture.rs 는 §3.1 밖).
- NOTE G7: 이 CHECK 는 awk 로 끝나므로 어떤 시험 묶음이 컴파일에 실패하면 그 묶음의 `test result:` 줄이 없어 FAILED_TOTAL=0 으로 읽힌다. 패킷 CHECK 를 그대로 두고, 같은 커밋에서 `bash -o pipefail` 로 cargo 종료 코드와 `test result:` 줄 수·통과 합을 따로 잰다(아래 결과 줄).
- NOTE G10 (Parent 처분 2, `outputs/v353-rhwp-table/RHWP_HWPX_SAVE_FIDELITY_L2_PACKET_DISPOSITION2.md`): 허용 목록에 `tests/hwpx_roundtrip_integration.rs`(c10 세 시험·c9 여백 시험의 기대값만)와 `src/parser/hwpx/header.rs`(switch case unit=CHAR 홀수 처리만)를 더했다. 근거 대조 — 한컴 12.30 MAC64 저장본 5개의 case unit=CHAR 값 43개가 모두 default = case×2+1(예외 0, `char-unit-evidence.json`).
