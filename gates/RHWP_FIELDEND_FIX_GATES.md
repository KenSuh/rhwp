# Gates: rhwp — 문단을 건너 끝나는 HWPX 필드 열기·왕복 (RHWP-FIELDEND-CROSS-PARAGRAPH-V380@r1)

- base `82e3da33f241558e09c3a21f24469d815b4aa758` · 필드 짝 커밋 `a969ffa8eac5bfcdd7c8c14e034072cca7589372` · 그림 없는 그림 커밋 `0e9274b24d7eec705001d2b3596afae901dbf6d2`(Root 결정 B 2026-11-16 · msg_b31f1f933598)
- 측정 폴더 `R=/Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix` — pin lane 도구 사본(`cond1_compare.py` 경로·대조 출처만 바꿈, `scripts/cond1_core.py` 는 pin lane 과 바이트 같음 · sha256 `7e0607a5…`), 코퍼스 6 양식(`corpus_sha256.txt` 대사는 도구가 v374 목록으로 한다), probe 두 벌(`src-base`=82e3da33 · `src-head`=0e9274b2, 도구가 git blob 전수 대사).
- 공통 — 아래 CHECK 는 워크트리 `/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix` 에서 돈다.

- [x] G1: head 에서 T5 원본(sha256 a0bd6914…)이 파싱 오류 0 으로 열린다(파서·DocumentCore 두 경로, probe `ir` 도 rc 0)
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph t5_opens_through_parser_and_document_core 2>&1 | grep -E "^test result" ; R=/Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix; $R/target-probe-head/debug/rhwp-gate-probe-head ir $R/corpus/T5.hwpx >/dev/null 2>&1; echo "head_ir_rc=$?"
  EXPECT: /test result: ok\. 1 passed; 0 failed[\s\S]*head_ir_rc=0/
  EVIDENCE: automatic-evidence=v1; definition-sha256=05d8aebdc1d3d3efe5c645e789c035bfc4ecba5c7c08f5470680637285f80f93; exit=0; EXPECT=matched; output-sha256=0bbf7a6337aa3de2d8985e39d117eef569c18c65956bea3a703f58bce106cd62; output-bytes=107; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=802bd5fa384c/47 entries

- [x] G2: 같은 열기 검사가 base 82e3da33 에서는 실패한다(대조 · 출력 보존 `$R/g2-base-t5-open.txt`)
  CHECK: R=/Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix; $R/target-probe-base/debug/rhwp-gate-probe-base ir $R/corpus/T5.hwpx >/dev/null 2>$R/g2-base-t5-open.txt; echo "base_ir_rc=$?"; cat $R/g2-base-t5-open.txt
  EXPECT: /base_ir_rc=1[\s\S]*fieldEnd beginIDRef에 대응하는 fieldBegin 없음/
  EVIDENCE: automatic-evidence=v1; definition-sha256=5136b26de3e3adc6bde3353bf7ec5b67aa34411b1275eb780851f8938317d4ce; exit=0; EXPECT=matched; output-sha256=beb221437d01783ffb5b5731c48693806e3892a63924d63bb6f7c15ef18adc43; output-bytes=177; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=802bd5fa384c/47 entries
  NOTE: 새 시험 3 건(T5 왕복·본문·셀)도 base 소스에서 같은 오류로 빨강이었다(구현 전 실행, `t5…`·`body…`·`cell…` FAILED · 짝 없는 end 거부·닫히지 않은 begin 시험 2 건은 base 에서도 초록).

- [x] G3: 원본 T5(수정 없음) 저장→재열기→재저장에서 필드 begin/end 한 쌍이 같은 문단 위치·같은 속성·같은 내용으로 남고, 두 번째 저장 section XML 이 첫 저장과 바이트 같다
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph t5_cross_paragraph_field_round_trips 2>&1 | grep -E "^test result"
  EXPECT: test result: ok. 1 passed; 0 failed
  EVIDENCE: automatic-evidence=v1; definition-sha256=e9faf29bc565a2e20cc045404bbf5dd9d8de52f4f08ea90abc7a8220551af077; exit=0; EXPECT=matched; output-sha256=4b9ed5a6144aa0d7ff9685481175c65edacf0cdd17250e4278e7eedfc299e6db; output-bytes=94; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=802bd5fa384c/47 entries
  NOTE: 첫 라운드(a969ffa8)에서는 원본 T5 저장이 필드와 무관한 그림 결함으로 거부돼 시험이 그 그림을 image1 로 바꿔 쟀다. 그 우회는 0e9274b2 에서 지웠고 이 시험은 원본 T5 바이트 그대로 돈다(그림 결함 수정은 G10).

- [x] G4: 문단을 건너는 필드가 본문 목록·표 셀 목록 모두에서 열리고, 저장→재열기→재저장에서 경계·글자 순서가 원본과 같다(시작 문단은 이 필드의 field_ranges 를 갖지 않는다)
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph spanning_paragraphs 2>&1 | grep -E "^test result"
  EXPECT: test result: ok. 2 passed; 0 failed
  EVIDENCE: automatic-evidence=v1; definition-sha256=c2fa08476ffad061ca979e4798781dbfb0c86ad170f73c1a8b416880c1649e69; exit=0; EXPECT=matched; output-sha256=dbe434dc09930a66cac33f42c37fcc06601ae0de704cb4fb7ee10260702d4434; output-bytes=94; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=802bd5fa384c/47 entries

- [x] G5: 짝 없는 fieldEnd 는 여전히 거부된다 — 어느 문단에도 begin 없음·end 가 begin 보다 앞 문단·같은 문단에서 end 가 앞·이미 닫힌 begin 에 end 재등장 4 경우와 e81c3b2 의 기존 거부 시험. 닫히지 않은 begin 은 열리되 저장이 거부된다
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph -- unmatched_field_end unclosed_field_begin 2>&1 | grep -E "^test result"; cargo test --offline --test hwpx_roundtrip_integration c8_native_field_binary_defaults_and_invalid_references_are_explicit 2>&1 | grep -E "^test result"
  EXPECT: /test result: ok\. 2 passed; 0 failed[\s\S]*test result: ok\. 1 passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=2d3f558cf2bf46360ac0a236a4761e9176b0b486af91220950b7fb8eed9d9680; exit=0; EXPECT=matched; output-sha256=914ecb354373b6b2bc722a51903dc6f298d2143596b85c7b2ef24f7bfb77c821; output-bytes=189; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=802bd5fa384c/47 entries

- [x] G6: 6 양식(G·T1·T2·T5·T6·T7) 조건 1 판정에서 base 82e3da33 보다 나빠진 양식 0, 열기 회귀 0, 대조 — base 판정이 pin lane 의 82e3da33 판정을 6/6 재현
  CHECK: cd /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix && python3 -B cond1_compare.py --base 82e3da33f241558e09c3a21f24469d815b4aa758 --head 0e9274b24d7eec705001d2b3596afae901dbf6d2 2>&1 | tail -3
  EXPECT: /pin_lane_82e3da33_verdict_match=6\/6[\s\S]*RHWP_OPEN forms=6 base_open=5 head_open=6 open_regress=0[\s\S]*RHWP_COND1 forms=6 base_pass=1 head_pass=2 regress=0/
  EVIDENCE: automatic-evidence=v1; definition-sha256=983ee8fb6921ae8223f497baa2d3f93a5fbd2a597c58f23d63dae505d6c3f002; exit=0; EXPECT=matched; output-sha256=9b3880fa9717396f0c1f86135cf43ea1fff78a2b6ea1c8058648b08cb652d035; output-bytes=193; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=802bd5fa384c/47 entries
  NOTE: 실행 기록 — 필드 짝 커밋 a969ffa8 대상 `20261116T105622Z`·`20261116T105842Z`(head_pass=1, T5 는 그림 저장 결함으로 fail) · 그림 커밋 0e9274b2 대상 `20261116T110600Z`(`$R/ac3-run-2.log`) — T5 fail→pass, T2 pass→pass, G/T1/T6/T7 fail→fail.
  NOTE: 대조 출처 = `/Users/ken/.multiagent/reports/v380-rhwp-l2-pin/work/20261116T031906Z/summary.json`(head=82e3da33f2…, 도구가 `pin['head'] == --base` 를 단언). 0e9274b2 에서 T5 는 열기·저장·재열기·재저장·최소 편집 전 과정을 통과한다.

- [x] G7: rhwp `cargo test` 전체 통과(개수)
  CHECK: cargo test --offline 2>&1 | grep -E "^test result:" | awk '{p+=$4; f+=$6; i+=$8} END {print "suites="NR" passed="p" failed="f" ignored="i}'
  EXPECT: /suites=19 passed=1261 failed=0 ignored=7/
  EVIDENCE: automatic-evidence=v1; definition-sha256=1169a1dcc194d810d2bc1981a8ef7dee0b3a4daced2b86102e6d7f10094e61a3; exit=0; EXPECT=matched; output-sha256=131f9ed90f4ad25693c7720318b46637ee5cff7f81634be3ee940efbcdc9efd8; output-bytes=41; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=802bd5fa384c/47 entries

- [x] G8: 변경한 파일 집합 = 필드 짝 소유 지점(파서) + 그 짝을 담는 모델 필드 + 그 짝을 쓰는 HWPX writer + 그림 없는 그림 writer·파서 참조 해석 + 새 시험 2 + 원장
  CHECK: git diff --name-only 82e3da33f241558e09c3a21f24469d815b4aa758..HEAD | sort | tr '\n' ' '
  EXPECT: gates/RHWP_FIELDEND_FIX_GATES.md src/model/control.rs src/parser/hwpx/section.rs src/serializer/hwpx/picture.rs src/serializer/hwpx/section.rs src/serializer/hwpx/shape.rs src/serializer/hwpx/table.rs tests/hwpx_field_cross_paragraph.rs tests/hwpx_imageless_picture.rs
  EVIDENCE: automatic-evidence=v1; definition-sha256=d9754f6ebfcf5e70885c14d2e35c9f3b585d4ddd2ae06da3e3b779a47f0e3e51; exit=0; EXPECT=matched; output-sha256=abacf518c019c66e20915eabea762177e9b6087bb3a917e974f06d2fd183aebd; output-bytes=269; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=802bd5fa384c/47 entries
  NOTE: 브리프 문구 «parser owner·새 시험·원장» 보다 넓다. 짝 판정은 파서 한 곳(`FieldPairing` · `src/parser/hwpx/section.rs`)에서 섹션 문단 목록 스택으로 닫지만, AC1 의 «저장→재열기에서 필드 보존» 은 writer 가 뒤 문단에 fieldEnd 를 쓸 수 있어야 해서 `HwpxField::span`(모델 1 필드)과 writer 3 파일(목록 단위 `later_field_ends` 전달)이 필요했다. 렌더러·필드 질의·편집 코드는 건드리지 않았다(시작 문단에 빈 범위를 두면 누름틀 안내문이 그려지므로 범위 없이 둔다 — 바이너리 파서의 기존 표현과 같다). Root 결정 B(2026-11-16)가 모델 span·writer 3 파일과 `src/serializer/hwpx/picture.rs` 를 MUST 로 받았다. 파서 `binaryItemIDRef` 해석(같은 파일 `src/parser/hwpx/section.rs`)은 빈 참조와 «숫자 없는 비어 있지 않은 참조» 를 bin_data_id 0 하나로 합치던 탓에 고쳤다 — 그대로 두면 미등록 참조 `"foo"` 가 저장에서 빈 참조로 바뀌어 거부 규칙이 느슨해진다(G10).

- [x] G9: 바뀐 파일에 clippy 경고 0
  CHECK: cargo clippy --offline --lib --tests 2>&1 | grep -E "^\s+--> " | grep -cE "src/parser/hwpx/section.rs|src/serializer/hwpx/(table|section|shape|picture).rs|src/model/control.rs|tests/hwpx_field_cross_paragraph.rs|tests/hwpx_imageless_picture.rs" | sed 's/^/CLIPPY_CHANGED_FILES=/'
  EXPECT: CLIPPY_CHANGED_FILES=0
  EVIDENCE: automatic-evidence=v1; definition-sha256=e1e2bd8b411c26fe40c0edc69ba535d8c442d6e55b0954f0f9f833ecb9cdb2ee; exit=0; EXPECT=matched; output-sha256=db48a9116b76159cbaf4c90e4e04518de2f4a47b9b1510fcb930450d48b0edf4; output-bytes=23; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=802bd5fa384c/47 entries

- [x] G10: 그림 없는 그림 개체(`binaryItemIDRef=""`)가 같은 빈 참조로 저장되고(새 bin 항목·다른 그림 참조 없음) 두 번째 저장이 바이트 같다 · 비어 있지 않은 미등록 참조(`image7`·`foo`·`image0`)는 여전히 저장 거부 · 원본 T5 가 파서·DocumentCore 두 경로로 저장되고 저장본이 다시 열린다(probe 저장→재열기 포함)
  CHECK: cargo test --offline --test hwpx_imageless_picture 2>&1 | grep -E "^test result"; R=/Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix; $R/target-probe-head/debug/rhwp-gate-probe-head save $R/corpus/T5.hwpx $R/g10-t5-saved.hwpx && $R/target-probe-head/debug/rhwp-gate-probe-head ir $R/g10-t5-saved.hwpx >/dev/null && echo T5_RAW_SAVE_REOPEN_OK
  EXPECT: /test result: ok\. 3 passed; 0 failed[\s\S]*T5_RAW_SAVE_REOPEN_OK/
  EVIDENCE: automatic-evidence=v1; definition-sha256=6c7de8f107f3506901784fd8ceaafb5e87bc5995c7ec9f32000835e1e625f285; exit=0; EXPECT=matched; output-sha256=c7bd73f9e95be9c6b80dd2cd514c664df51af3912baa263c9afff97616b797d2; output-bytes=152; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=802bd5fa384c/47 entries
  NOTE: 빨강 대조 — 같은 시험 파일을 직전 커밋 767d0cfc 트리에 넣어 돌리면 `test result: FAILED. 0 passed; 3 failed`(`$R/picture-redcheck-767d0cf.txt`). 필드 짝 보존은 G3 이 원본 T5 로 잰다.
