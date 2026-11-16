# Gates: rhwp — 문단을 건너 끝나는 HWPX 필드 열기·왕복 (RHWP-FIELDEND-CROSS-PARAGRAPH-V380@r1)

- base `82e3da33f241558e09c3a21f24469d815b4aa758` · 구현 커밋 `a969ffa8eac5bfcdd7c8c14e034072cca7589372`
- 측정 폴더 `R=/Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix` — pin lane 도구 사본(`cond1_compare.py` 경로·대조 출처만 바꿈, `scripts/cond1_core.py` 는 pin lane 과 바이트 같음 · sha256 `7e0607a5…`), 코퍼스 6 양식(`corpus_sha256.txt` 대사는 도구가 v374 목록으로 한다), probe 두 벌(`src-base`=82e3da33 · `src-head`=a969ffa8, 도구가 git blob 전수 대사).
- 공통 — 아래 CHECK 는 워크트리 `/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix` 에서 돈다.

- [ ] G1: head 에서 T5 원본(sha256 a0bd6914…)이 파싱 오류 0 으로 열린다(파서·DocumentCore 두 경로, probe `ir` 도 rc 0)
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph t5_opens_through_parser_and_document_core 2>&1 | grep -E "^test result" ; R=/Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix; $R/target-probe-head/debug/rhwp-gate-probe-head ir $R/corpus/T5.hwpx >/dev/null 2>&1; echo "head_ir_rc=$?"
  EXPECT: /test result: ok\. 1 passed; 0 failed[\s\S]*head_ir_rc=0/
  EVIDENCE: pending

- [ ] G2: 같은 열기 검사가 base 82e3da33 에서는 실패한다(대조 · 출력 보존 `$R/g2-base-t5-open.txt`)
  CHECK: R=/Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix; $R/target-probe-base/debug/rhwp-gate-probe-base ir $R/corpus/T5.hwpx >/dev/null 2>$R/g2-base-t5-open.txt; echo "base_ir_rc=$?"; cat $R/g2-base-t5-open.txt
  EXPECT: /base_ir_rc=1[\s\S]*fieldEnd beginIDRef에 대응하는 fieldBegin 없음/
  EVIDENCE: pending
  NOTE: 새 시험 3 건(T5 왕복·본문·셀)도 base 소스에서 같은 오류로 빨강이었다(구현 전 실행, `t5…`·`body…`·`cell…` FAILED · 짝 없는 end 거부·닫히지 않은 begin 시험 2 건은 base 에서도 초록).

- [ ] G3: T5 저장→재열기→재저장에서 필드 begin/end 한 쌍이 같은 문단 위치·같은 속성·같은 내용으로 남고, 두 번째 저장 section XML 이 첫 저장과 바이트 같다
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph t5_cross_paragraph_field_round_trips 2>&1 | grep -E "^test result"
  EXPECT: test result: ok. 1 passed; 0 failed
  EVIDENCE: pending
  NOTE: T5 전체 저장은 필드와 무관한 기존 결함으로 거부된다 — 두 번째 `<hp:pic>` 이 `binaryItemIDRef=""`(그림 없는 그림 개체)라 `<hp:pic> binaryItemIDRef 미등록 bin_data_id=0` (base a34d88ff·82e3da33·head 모두 같은 오류 · `$R/checks/c1_20261116T105622Z_head_T5_save_s1.stderr`). 이 시험은 그 그림 하나만 같은 문서의 image1 로 가리킨 뒤 필드 왕복을 잰다. 그림 결함은 이 lane 범위(필드 짝 판정) 밖이다.

- [ ] G4: 문단을 건너는 필드가 본문 목록·표 셀 목록 모두에서 열리고, 저장→재열기→재저장에서 경계·글자 순서가 원본과 같다(시작 문단은 이 필드의 field_ranges 를 갖지 않는다)
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph spanning_paragraphs 2>&1 | grep -E "^test result"
  EXPECT: test result: ok. 2 passed; 0 failed
  EVIDENCE: pending

- [ ] G5: 짝 없는 fieldEnd 는 여전히 거부된다 — 어느 문단에도 begin 없음·end 가 begin 보다 앞 문단·같은 문단에서 end 가 앞·이미 닫힌 begin 에 end 재등장 4 경우와 e81c3b2 의 기존 거부 시험. 닫히지 않은 begin 은 열리되 저장이 거부된다
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph -- unmatched_field_end unclosed_field_begin 2>&1 | grep -E "^test result"; cargo test --offline --test hwpx_roundtrip_integration c8_native_field_binary_defaults_and_invalid_references_are_explicit 2>&1 | grep -E "^test result"
  EXPECT: /test result: ok\. 2 passed; 0 failed[\s\S]*test result: ok\. 1 passed; 0 failed/
  EVIDENCE: pending

- [ ] G6: 6 양식(G·T1·T2·T5·T6·T7) 조건 1 판정에서 base 82e3da33 보다 나빠진 양식 0, 열기 회귀 0, 대조 — base 판정이 pin lane 의 82e3da33 판정을 6/6 재현
  CHECK: cd /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix && python3 -B cond1_compare.py --base 82e3da33f241558e09c3a21f24469d815b4aa758 --head a969ffa8eac5bfcdd7c8c14e034072cca7589372 2>&1 | tail -3
  EXPECT: /pin_lane_82e3da33_verdict_match=6\/6[\s\S]*RHWP_OPEN forms=6 base_open=5 head_open=6 open_regress=0[\s\S]*RHWP_COND1 forms=6 base_pass=1 head_pass=1 regress=0/
  EVIDENCE: pending
  NOTE: 대조 출처 = `/Users/ken/.multiagent/reports/v380-rhwp-l2-pin/work/20261116T031906Z/summary.json`(head=82e3da33f2…, 도구가 `pin['head'] == --base` 를 단언). T5 head 는 이제 열리지만 조건 1 은 G3 NOTE 의 그림 저장 결함으로 fail 로 남는다.

- [ ] G7: rhwp `cargo test` 전체 통과(개수)
  CHECK: cargo test --offline 2>&1 | grep -E "^test result:" | awk '{p+=$4; f+=$6; i+=$8} END {print "suites="NR" passed="p" failed="f" ignored="i}'
  EXPECT: /suites=18 passed=1258 failed=0 ignored=7/
  EVIDENCE: pending

- [ ] G8: 변경한 파일 집합 = 필드 짝 소유 지점(파서) + 그 짝을 담는 모델 필드 + 그 짝을 쓰는 HWPX writer + 새 시험 + 원장
  CHECK: git diff --name-only 82e3da33f241558e09c3a21f24469d815b4aa758..HEAD | sort | tr '\n' ' '
  EXPECT: gates/RHWP_FIELDEND_FIX_GATES.md src/model/control.rs src/parser/hwpx/section.rs src/serializer/hwpx/section.rs src/serializer/hwpx/shape.rs src/serializer/hwpx/table.rs tests/hwpx_field_cross_paragraph.rs
  EVIDENCE: pending
  NOTE: 브리프 문구 «parser owner·새 시험·원장» 보다 넓다. 짝 판정은 파서 한 곳(`FieldPairing` · `src/parser/hwpx/section.rs`)에서 섹션 문단 목록 스택으로 닫지만, AC1 의 «저장→재열기에서 필드 보존» 은 writer 가 뒤 문단에 fieldEnd 를 쓸 수 있어야 해서 `HwpxField::span`(모델 1 필드)과 writer 3 파일(목록 단위 `later_field_ends` 전달)이 필요했다. 렌더러·필드 질의·편집 코드는 건드리지 않았다(시작 문단에 빈 범위를 두면 누름틀 안내문이 그려지므로 범위 없이 둔다 — 바이너리 파서의 기존 표현과 같다). 리뷰가 범위를 판정한다.

- [ ] G9: 바뀐 파일에 clippy 경고 0
  CHECK: cargo clippy --offline --lib --tests 2>&1 | grep -E "^\s+--> " | grep -cE "src/parser/hwpx/section.rs|src/serializer/hwpx/(table|section|shape).rs|src/model/control.rs|tests/hwpx_field_cross_paragraph.rs" | sed 's/^/CLIPPY_CHANGED_FILES=/'
  EXPECT: CLIPPY_CHANGED_FILES=0
  EVIDENCE: pending
