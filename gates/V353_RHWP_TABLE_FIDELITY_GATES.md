# Gates: v353 rhwp table fidelity P0–P2

- [ ] G1: 변경 전 baseline — T1·T2 왕복 손실이 0 이 아님을 수정 전 커밋(a34d88ff)에서 기록했다
  CHECK: python3 -c 'import json;d=json.load(open(".omo/evidence/v353-rhwp-table-fidelity/baseline.json"));assert d["head"]=="a34d88ff3bae317ad08d8f0b99504e4395278e6a";assert d["T1"]["ATTR_LOSS_TOTAL"]>0 and d["T2"]["ATTR_LOSS_TOTAL"]>0;print("BASELINE_OK",d["T1"]["ATTR_LOSS_TOTAL"],d["T2"]["ATTR_LOSS_TOTAL"])'
  EXPECT: /^BASELINE_OK \d+ \d+$/m
  EVIDENCE: pending

- [ ] G2: 속성 보존 — 수정 head 에서 T1·T2·T6·T7 왕복 손실 합계 0(lineWrap·noAdjust·lock·protect·pos.*·sz.*·textFlow·pageBreak)
  CHECK: python3 tools/roundtrip_diff.py --fixtures tests/fixtures/table-fidelity --cases T1,T2,T6,T7 --summary
  EXPECT: /^ATTR_LOSS_TOTAL=0$/m
  EVIDENCE: pending

- [ ] G3: 속성 보존 단위 시험(noAdjust·lock·sz protect·allowOverlap·flowWithText·textFlow·셀 lineWrap SQUEEZE·셀 protect 각 1 사례 이상)
  CHECK: cargo test --lib serializer::hwpx::table 2>&1 | tail -3
  EXPECT: /test result: ok\. [1-9]\d* passed; 0 failed/
  EVIDENCE: pending

- [ ] G4: pageBreak 4값 — HWPX→IR→HWPX 와 HWPX→HWP5→HWPX 두 경로 모두 원 문자열 보존
  CHECK: cargo test --test hwpx_table_fidelity page_break 2>&1 | tail -3
  EXPECT: /test result: ok\. [2-9]\d* passed; 0 failed/
  EVIDENCE: pending
  NOTE: Parent 처분 2(선택지 A, 2026-10-25 ~10:20Z, `/Users/ken/ken wiki/outputs/v353-rhwp-table/RHWP_TABLE_FIDELITY_P0_P2_PACKET_DISPOSITION2.md`)에 따른 해석이다. TABLE·CELL·NONE 은 HWPX→IR→HWPX 와 HWPX→HWP5→HWPX 두 경로에서 원 문자열을 보존한다. ROW 는 «RowBreak 로 읽고 CELL 로 정규화»를 시험한다. ROW 의 문자 그대로 보존은 HWP5 형식(쪽 나눔 2비트)상 불가능하므로 요구하지 않는다.

- [ ] G5: 표시 높이 — T6(표시 높이 ≠ 셀 합) 행 삽입·삭제·텍스트 편집 뒤 sz.height 가 §3.2 공식대로 유지
  CHECK: cargo test --test hwpx_table_fidelity display_height 2>&1 | tail -3
  EXPECT: /test result: ok\. [3-9]\d* passed; 0 failed/
  EVIDENCE: pending

- [ ] G6: 폭 공식 — cellSpacing>0 표와 병합 없는 표에서 sz.width == Σ열 폭 + (colCnt+1)×cellSpacing
  CHECK: cargo test --test hwpx_table_fidelity width_formula 2>&1 | tail -3
  EXPECT: /test result: ok\. [1-9]\d* passed; 0 failed/
  EVIDENCE: pending

- [ ] G7: 음성 대조 — G4·G5 시험을 수정 전 a34d88ff 의 해당 함수에 적용하면 실패한다(시험이 결함을 실제로 잡는다)
  CHECK: python3 -c 'import json;d=json.load(open(".omo/evidence/v353-rhwp-table-fidelity/negative-control.json"));assert d["base"]=="a34d88ff3bae317ad08d8f0b99504e4395278e6a";assert d["page_break_failed"]>0 and d["display_height_failed"]>0;print("NEG_OK")'
  EXPECT: /^NEG_OK$/m
  EVIDENCE: pending

- [ ] G8: 전체 시험
  CHECK: cargo test 2>&1 | grep -E "^test result:" | awk '{f+=$6} END {print "FAILED_TOTAL=" f+0}'
  EXPECT: /^FAILED_TOTAL=0$/m
  EVIDENCE: pending

- [ ] G9: clippy
  CHECK: cargo clippy -- -D warnings 2>&1 | tail -1
  EXPECT: /Finished/
  EVIDENCE: pending

- [ ] G10: WASM 빌드(에디터가 쓰는 대상)
  CHECK: cargo build --lib --target wasm32-unknown-unknown --release 2>&1 | tail -1
  EXPECT: /Finished/
  EVIDENCE: pending

- [ ] G11: 범위 — base 대비 변경 파일이 전부 §3.1 MUST 안
  CHECK: git diff --name-only a34d88ff3bae317ad08d8f0b99504e4395278e6a HEAD | grep -vE '^(src/model/table\.rs|src/model/table/tests\.rs|src/model/shape\.rs|src/parser/hwpx/section\.rs|src/parser/control\.rs|src/serializer/hwpx/table\.rs|src/serializer/control\.rs|src/document_core/commands/(table_ops|text_editing)\.rs|src/document_core/converters/common_obj_attr_writer\.rs|tools/roundtrip_diff\.py|tests/hwpx_table_fidelity\.rs|tests/fixtures/table-fidelity/.*|gates/V353_RHWP_TABLE_FIDELITY_GATES\.md|\.omo/evidence/v353-rhwp-table-fidelity/.*|\.gitignore)$' | wc -l | tr -d ' ' | sed 's/^/OUT_OF_SCOPE=/'
  EXPECT: /^OUT_OF_SCOPE=0$/m
  EVIDENCE: pending
  NOTE: `src/document_core/converters/common_obj_attr_writer.rs` 는 Parent 처분 1(선택지 A, 2026-10-25 ~10:10Z, `/Users/ken/ken wiki/outputs/v353-rhwp-table/RHWP_TABLE_FIDELITY_P0_P2_PACKET_DISPOSITION1.md`)로 허용 목록에 더했다. 허용 범위는 `#[cfg(test)] make_sample()` 리터럴의 `..Default::default()` 한 줄 추가뿐이다.

- [ ] G12: 한컴 확인용 산출물 — T1·T6 의 편집 전·후 HWPX 를 .omo/evidence/v353-rhwp-table-fidelity/hancom/ 에 저장(한컴 열기·시각 확인은 Parent 가 한다)
  CHECK: ls .omo/evidence/v353-rhwp-table-fidelity/hancom/*.hwpx | wc -l | tr -d ' ' | sed 's/^/HANCOM_FILES=/'
  EXPECT: /^HANCOM_FILES=[4-9]$/m
  EVIDENCE: pending
