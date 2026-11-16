# Gates: rhwp — 문단을 건너 끝나는 HWPX 필드 열기·왕복 (RHWP-FIELDEND-CROSS-PARAGRAPH-V380@r1)

- base `82e3da33f241558e09c3a21f24469d815b4aa758` · 필드 짝 커밋 `a969ffa8eac5bfcdd7c8c14e034072cca7589372` · 그림 없는 그림 커밋 `0e9274b24d7eec705001d2b3596afae901dbf6d2`(Root 결정 B 2026-11-16 · msg_b31f1f933598)
- 초기 측정(이력) 폴더 `R=/Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix` — pin lane 도구 사본(`cond1_compare.py` 경로·대조 출처만 바꿈, `scripts/cond1_core.py` 는 pin lane 과 바이트 같음 · sha256 `7e0607a5…`), 코퍼스 6 양식(`corpus_sha256.txt` 대사는 도구가 v374 목록으로 한다), probe 두 벌(`src-base`=82e3da33 · `src-head`=0e9274b2, 도구가 git blob 전수 대사).
- 공통 — 아래 CHECK 는 워크트리 `/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix` 에서 돈다.

- C1 product checkpoint `fc35a04d48a557a12729be69df998d45d72a2d95`. G6/G7/G9 carry the final-source runs across subsequent ledger-only commits; the final checker run also repeated them because free-form evidence was not treated as current automatic evidence. Class guards are rerun after the correction. New source scope includes Paragraph edit owners and Root approval msg_dd0ab753ae6e for image.rs and mechanical Picture initializers.

- [x] G1: head 에서 T5 원본(sha256 a0bd6914…)이 파싱 오류 0 으로 열린다(파서·DocumentCore 두 경로, probe `ir` 도 rc 0)
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph t5_opens_through_parser_and_document_core && /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c1-ac3/target-probe-head/debug/rhwp-gate-probe-head ir /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/corpus/T5.hwpx >/dev/null && echo head_ir_rc=0
  EXPECT: /test result: ok\. 1 passed; 0 failed[\s\S]*head_ir_rc=0/
  EVIDENCE: automatic-evidence=v1; definition-sha256=6bcc2208150b39553ff165fc5f878a076978aa3b0c78c4f692320aa927b0a3f0; exit=0; EXPECT=matched; output-sha256=48a66d90488fd07aafec6fd89872c424c834b434a352a46171cad6ef5ce1aec3; output-bytes=367; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=4f045aa37ddf/49 entries

- [x] G2: 같은 열기 검사가 base 82e3da33 에서는 실패한다(대조 · 출력 보존 `$R/g2-base-t5-open.txt`)
  CHECK: R=/Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix; $R/target-probe-base/debug/rhwp-gate-probe-base ir $R/corpus/T5.hwpx >/dev/null 2>$R/g2-base-t5-open.txt; echo "base_ir_rc=$?"; cat $R/g2-base-t5-open.txt
  EXPECT: /base_ir_rc=1[\s\S]*fieldEnd beginIDRef에 대응하는 fieldBegin 없음/
  EVIDENCE: automatic-evidence=v1; definition-sha256=5136b26de3e3adc6bde3353bf7ec5b67aa34411b1275eb780851f8938317d4ce; exit=0; EXPECT=matched; output-sha256=beb221437d01783ffb5b5731c48693806e3892a63924d63bb6f7c15ef18adc43; output-bytes=177; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=4f045aa37ddf/49 entries
  NOTE: 새 시험 3 건(T5 왕복·본문·셀)도 base 소스에서 같은 오류로 빨강이었다(구현 전 실행, `t5…`·`body…`·`cell…` FAILED · 짝 없는 end 거부·닫히지 않은 begin 시험 2 건은 base 에서도 초록).

- [x] G3: 원본 T5(수정 없음) 저장→재열기→재저장에서 필드 begin/end 한 쌍이 같은 문단 위치·같은 속성·같은 내용으로 남고, 두 번째 저장 section XML 이 첫 저장과 바이트 같다
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph t5_cross_paragraph_field_round_trips
  EXPECT: test result: ok. 1 passed; 0 failed
  EVIDENCE: automatic-evidence=v1; definition-sha256=8faf6b09f5707fbe29c5abb9344fad58e06ceac2c4d69d5e3ede70a3346f267d; exit=0; EXPECT=matched; output-sha256=c89e4d27f451fc84c3c1edcfb59ba1a905520e8d8daf20f8d3ddf945df1fa54b; output-bytes=349; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=4f045aa37ddf/49 entries
  NOTE: 첫 라운드(a969ffa8)에서는 원본 T5 저장이 필드와 무관한 그림 결함으로 거부돼 시험이 그 그림을 image1 로 바꿔 쟀다. 그 우회는 0e9274b2 에서 지웠고 이 시험은 원본 T5 바이트 그대로 돈다(그림 결함 수정은 G10).

- [x] G4: 문단을 건너는 필드가 본문 목록·표 셀 목록 모두에서 열리고, 저장→재열기→재저장에서 경계·글자 순서가 원본과 같다(시작 문단은 이 필드의 field_ranges 를 갖지 않는다)
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph spanning_paragraphs
  EXPECT: test result: ok. 2 passed; 0 failed
  EVIDENCE: automatic-evidence=v1; definition-sha256=4cd87ef1f000735faac44e491ba04ec7e401f55349bdff2116ff4f18ffaee6e0; exit=0; EXPECT=matched; output-sha256=c94085ccf25c15c3586e01ba0e0950bb777a4f7491acd42a78bc3b59adc35354; output-bytes=421; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=4f045aa37ddf/49 entries

- [x] G5: 짝 없는 fieldEnd 는 여전히 거부된다 — 어느 문단에도 begin 없음·end 가 begin 보다 앞 문단·같은 문단에서 end 가 앞·이미 닫힌 begin 에 end 재등장 4 경우와 e81c3b2 의 기존 거부 시험. 닫히지 않은 begin 은 열리되 저장이 거부된다
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph -- unmatched_field_end unclosed_field_begin && cargo test --offline --test hwpx_roundtrip_integration c8_native_field_binary_defaults_and_invalid_references_are_explicit
  EXPECT: /test result: ok\. 2 passed; 0 failed[\s\S]*test result: ok\. 1 passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=bdeccc6bf1e92276dfe6298f7e87968d30143f231ac780d806aabed341324ea1; exit=0; EXPECT=matched; output-sha256=b705a8c5a119ea1a54659e379ae6be5f47186c8fb02d77f3bc63b358e11cde7f; output-bytes=811; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=4f045aa37ddf/49 entries

- [x] G6: 6 양식(G·T1·T2·T5·T6·T7) 조건 1 판정에서 base 82e3da33 보다 나빠진 양식 0, 열기 회귀 0, 대조 — base 판정이 pin lane 의 82e3da33 판정을 6/6 재현
  CHECK: cd /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c1-ac3 && python3 -B cond1_compare.py --base 82e3da33f241558e09c3a21f24469d815b4aa758 --head fc35a04d48a557a12729be69df998d45d72a2d95
  EXPECT: /pin_lane_82e3da33_verdict_match=6\/6[\s\S]*RHWP_OPEN forms=6 base_open=5 head_open=6 open_regress=0[\s\S]*RHWP_COND1 forms=6 base_pass=1 head_pass=2 regress=0/
  EVIDENCE: automatic-evidence=v1; definition-sha256=18399329fb066e64f92a29f270fc706847d0df1d3363f5d471627a34761c7c6b; exit=0; EXPECT=matched; output-sha256=21da185abb8cb16b533dcabcdd04d780db400d6fb903c31665eacadafafc24b7; output-bytes=731; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=4f045aa37ddf/49 entries
  NOTE: 실행 기록 — 필드 짝 커밋 a969ffa8 대상 `20261116T105622Z`·`20261116T105842Z`(head_pass=1, T5 는 그림 저장 결함으로 fail) · 그림 커밋 0e9274b2 대상 `20261116T110600Z`(`$R/ac3-run-2.log`) — T5 fail→pass, T2 pass→pass, G/T1/T6/T7 fail→fail.
  NOTE: 대조 출처 = `/Users/ken/.multiagent/reports/v380-rhwp-l2-pin/work/20261116T031906Z/summary.json`(head=82e3da33f2…, 도구가 `pin['head'] == --base` 를 단언). 0e9274b2 에서 T5 는 열기·저장·재열기·재저장·최소 편집 전 과정을 통과한다.

  NOTE: C1 run 20261116T120042Z tests exact product checkpoint fc35a04d48a557a12729be69df998d45d72a2d95. Comparator and cond1_core.py are byte-identical copies in c1-ac3; prior snapshots and report tools were preserved. G/T1/T6/T7 remain pre-existing failures; T2 pass and T5 pass.

- [x] G7: rhwp `cargo test` 전체 통과(개수)
  CHECK: bash -o pipefail -c 'cargo test --offline 2>&1 | awk "/^test result:/{p+=\$4; f+=\$6; i+=\$8; s++} END{print \"suites=\"s\" passed=\"p\" failed=\"f\" ignored=\"i}"'
  EXPECT: suites=19 passed=1268 failed=0 ignored=7
  EVIDENCE: automatic-evidence=v1; definition-sha256=c6b6cb64228b5c4dc43b17bed7a558be50a9abb7ab6a1022bb6e9b36ca3dc699; exit=0; EXPECT=matched; output-sha256=5e0c2d07b028f40664fdd9df4d2dba726b66073ed323b4d0a0098753346f0368; output-bytes=41; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=4f045aa37ddf/49 entries

- [x] G8: 변경 파일 14개가 initial MUST + correction edit owners + Root-approved image.rs/Picture initializer 범위 안이다
  CHECK: git diff --name-only 82e3da33f241558e09c3a21f24469d815b4aa758..HEAD | sort | tr '\n' ' '
  EXPECT: gates/RHWP_FIELDEND_FIX_GATES.md src/document_core/commands/document.rs src/document_core/queries/field_query.rs src/model/control.rs src/model/image.rs src/model/paragraph.rs src/parser/hwpx/section.rs src/serializer/hwpx/picture.rs src/serializer/hwpx/section.rs src/serializer/hwpx/shape.rs src/serializer/hwpx/table.rs src/wasm_api/tests.rs tests/hwpx_field_cross_paragraph.rs tests/hwpx_imageless_picture.rs
  EVIDENCE: automatic-evidence=v1; definition-sha256=bf1424feae1c4a2b4e76b1049b50d76a8005c57b15cc214df29d5bff83600d9e; exit=0; EXPECT=matched; output-sha256=0b497b76a88091901679919ca195c0a12ffa9278a4a4bcbd916fe7b91c239647; output-bytes=413; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=4f045aa37ddf/49 entries

NOTE: G8 C1 범위 — HwpxField::span 을 삭제하고 Paragraph::hwpx_field_markers 가 교차 문단 경계를 소유한다. 직접 field_ranges 를 옮기는 document.rs·field_query.rs 도 같은 shift helper 에 연결했다. image.rs 와 wasm_api/tests.rs 의 두 Picture 초기화는 Root msg_dd0ab753ae6e 승인 범위다.

- [x] G9: 기능 변경 파일에 clippy 경고 0; wasm_api/tests.rs 의 기존 경고는 두 기계적 초기화 변경과 분리해 carry
  CHECK: cargo clippy --offline --lib --tests > /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c1-clippy-gate.log 2>&1 && awk '/^[[:space:]]*--> (src\/model\/(control|image|paragraph)\.rs|src\/parser\/hwpx\/section\.rs|src\/serializer\/hwpx\/(table|section|shape|picture)\.rs|src\/document_core\/(commands\/document|queries\/field_query)\.rs|tests\/hwpx_(field_cross_paragraph|imageless_picture)\.rs)/{n++} END{print "CLIPPY_CHANGED_CODE_WARNINGS="n+0; exit(n>0)}' /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c1-clippy-gate.log
  EXPECT: CLIPPY_CHANGED_CODE_WARNINGS=0
  EVIDENCE: automatic-evidence=v1; definition-sha256=7ab62daef591528e0072ee38141f6cf51ac2ce1f4b7e623dd283be0ca2355586; exit=0; EXPECT=matched; output-sha256=1756e7f07aced73d1dc321aac152c855522ee6984420c5a3a1fb4034c579d8f2; output-bytes=31; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=4f045aa37ddf/49 entries

- [x] G10: 그림 없는 그림 개체(`binaryItemIDRef=""`)가 같은 빈 참조로 저장되고(새 bin 항목·다른 그림 참조 없음) 두 번째 저장이 바이트 같다 · 비어 있지 않은 미등록 참조(`image7`·`foo`·`image0`)는 여전히 저장 거부 · 원본 T5 가 파서·DocumentCore 두 경로로 저장되고 저장본이 다시 열린다(probe 저장→재열기 포함)
  CHECK: cargo test --offline --test hwpx_imageless_picture && /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c1-ac3/target-probe-head/debug/rhwp-gate-probe-head save /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/corpus/T5.hwpx /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c1-t5-raw-saved.hwpx && /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c1-ac3/target-probe-head/debug/rhwp-gate-probe-head ir /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c1-t5-raw-saved.hwpx >/dev/null && echo T5_RAW_SAVE_REOPEN_OK
  EXPECT: /test result: ok\. 4 passed; 0 failed[\s\S]*T5_RAW_SAVE_REOPEN_OK/
  EVIDENCE: automatic-evidence=v1; definition-sha256=c2542f0d0e22116435ab8b13356a83d4b6c9e0dd401dedb0bb740b188ad32c50; exit=0; EXPECT=matched; output-sha256=2cef8ea8def822364da701ea3b41505a3aa9e449ce459a5057818531bcae96aa; output-bytes=635; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=4f045aa37ddf/49 entries
  NOTE: 빨강 대조 — 같은 시험 파일을 직전 커밋 767d0cfc 트리에 넣어 돌리면 `test result: FAILED. 0 passed; 3 failed`(`$R/picture-redcheck-767d0cf.txt`). 필드 짝 보존은 G3 이 원본 T5 로 잰다.

- [x] G11: RHWPFIX-C1 — field endpoints follow insert/delete/split/merge; deleted or reversed endpoints refuse with HWPX_FIELD_ENDPOINT; each successful second save is byte-identical
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph c1_
  EXPECT: /test result: ok\. 4 passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=76e5fa69ca0ae98dabc2f11fe53f9891d4bd2b4b2028d165260484316c90ea42; exit=0; EXPECT=matched; output-sha256=38376a235c11d60f0629c2187f8638b9cfd7b89c243052c9e7a1d90339b542d0; output-bytes=565; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=4f045aa37ddf/49 entries

- [x] G12: RHWPFIX-C2 — cross-paragraph acceptance equals serialized lists including ancestor reachability; every list refuses unclosed begins at open or save
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph c2_
  EXPECT: /test result: ok\. 2 passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=9ad5a6424c51e2fefc55897151af612535c8434ae45fea817419f587e6291610; exit=0; EXPECT=matched; output-sha256=c479428a3abccb33916ef15a00e15bc3c4f2cc84a183fa63fdc30177d53c901c; output-bytes=430; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=4f045aa37ddf/49 entries

- [x] G13: RHWPFIX-C3 — missing picture reference refuses save; explicitly empty and registered numbered references keep existing round trips; unregistered references refuse
  CHECK: cargo test --offline --test hwpx_imageless_picture
  EXPECT: /test result: ok\. 4 passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=66fa991bab64aa3c2bc6652888c5dd73f21c58d18e46af72efda2d480841edda; exit=0; EXPECT=matched; output-sha256=e113ca279c82f532eb021c53f83213f900242c6c9820bf1f309f902100002bd5; output-bytes=577; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=4f045aa37ddf/49 entries
