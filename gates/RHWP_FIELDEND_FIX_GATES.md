# Gates: rhwp — 문단을 건너 끝나는 HWPX 필드 열기·왕복 (RHWP-FIELDEND-CROSS-PARAGRAPH-V380@r1)

- base `82e3da33f241558e09c3a21f24469d815b4aa758` · 필드 짝 커밋 `a969ffa8eac5bfcdd7c8c14e034072cca7589372` · 그림 없는 그림 커밋 `0e9274b24d7eec705001d2b3596afae901dbf6d2`(Root 결정 B 2026-11-16 · msg_b31f1f933598)
- 초기 측정(이력) 폴더 `R=/Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix` — pin lane 도구 사본(`cond1_compare.py` 경로·대조 출처만 바꿈, `scripts/cond1_core.py` 는 pin lane 과 바이트 같음 · sha256 `7e0607a5…`), 코퍼스 6 양식(`corpus_sha256.txt` 대사는 도구가 v374 목록으로 한다), probe 두 벌(`src-base`=82e3da33 · `src-head`=0e9274b2, 도구가 git blob 전수 대사).
- 공통 — 아래 CHECK 는 워크트리 `/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix` 에서 돈다.

- C1 product checkpoint `fc35a04d48a557a12729be69df998d45d72a2d95`. G6/G7/G9 carry the final-source runs across subsequent ledger-only commits; the final checker run also repeated them because free-form evidence was not treated as current automatic evidence. Class guards are rerun after the correction. New source scope includes Paragraph edit owners and Root approval msg_dd0ab753ae6e for image.rs and mechanical Picture initializers.

- [x] G1: head 에서 T5 원본(sha256 a0bd6914…)이 파싱 오류 0 으로 열린다(파서·DocumentCore 두 경로, probe `ir` 도 rc 0)
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph t5_opens_through_parser_and_document_core && /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c2-ac3/target-probe-head/debug/rhwp-gate-probe-head ir /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/corpus/T5.hwpx >/dev/null && echo head_ir_rc=0
  EXPECT: /test result: ok\. 1 passed; 0 failed[\s\S]*head_ir_rc=0/
  EVIDENCE: automatic-evidence=v1; definition-sha256=88d7c88bd214de897cac472e3bb06ebb32d9fb3f6b73aba9a7431ccfd1342800; exit=0; EXPECT=matched; output-sha256=60cb50df7a9c8a444bc9d1c73cc9b8486badcd3d4e85ac26977fdbf8d5f5490a; output-bytes=367; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=58b3c64490f0/49 entries

- [x] G2: 같은 열기 검사가 base 82e3da33 에서는 실패한다(대조 · 출력 보존 `$R/g2-base-t5-open.txt`)
  CHECK: R=/Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix; $R/target-probe-base/debug/rhwp-gate-probe-base ir $R/corpus/T5.hwpx >/dev/null 2>$R/g2-base-t5-open.txt; echo "base_ir_rc=$?"; cat $R/g2-base-t5-open.txt
  EXPECT: /base_ir_rc=1[\s\S]*fieldEnd beginIDRef에 대응하는 fieldBegin 없음/
  EVIDENCE: automatic-evidence=v1; definition-sha256=5136b26de3e3adc6bde3353bf7ec5b67aa34411b1275eb780851f8938317d4ce; exit=0; EXPECT=matched; output-sha256=beb221437d01783ffb5b5731c48693806e3892a63924d63bb6f7c15ef18adc43; output-bytes=177; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=4f045aa37ddf/49 entries
  NOTE: 새 시험 3 건(T5 왕복·본문·셀)도 base 소스에서 같은 오류로 빨강이었다(구현 전 실행, `t5…`·`body…`·`cell…` FAILED · 짝 없는 end 거부·닫히지 않은 begin 시험 2 건은 base 에서도 초록).

- [x] G3: 원본 T5(수정 없음) 저장→재열기→재저장에서 필드 begin/end 한 쌍이 같은 문단 위치·같은 속성·같은 내용으로 남고, 두 번째 저장 section XML 이 첫 저장과 바이트 같다
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph t5_cross_paragraph_field_round_trips
  EXPECT: test result: ok. 1 passed; 0 failed
  EVIDENCE: automatic-evidence=v1; definition-sha256=8faf6b09f5707fbe29c5abb9344fad58e06ceac2c4d69d5e3ede70a3346f267d; exit=0; EXPECT=matched; output-sha256=eed2f09f3a42d8b95dbd69c7edf759a2346ba14334ff109dda24799c887b9686; output-bytes=349; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=58b3c64490f0/49 entries
  NOTE: 첫 라운드(a969ffa8)에서는 원본 T5 저장이 필드와 무관한 그림 결함으로 거부돼 시험이 그 그림을 image1 로 바꿔 쟀다. 그 우회는 0e9274b2 에서 지웠고 이 시험은 원본 T5 바이트 그대로 돈다(그림 결함 수정은 G10).

- [x] G4: 문단을 건너는 필드가 본문 목록·표 셀 목록 모두에서 열리고, 저장→재열기→재저장에서 경계·글자 순서가 원본과 같다(시작 문단은 이 필드의 field_ranges 를 갖지 않는다)
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph spanning_paragraphs
  EXPECT: test result: ok. 2 passed; 0 failed
  EVIDENCE: automatic-evidence=v1; definition-sha256=4cd87ef1f000735faac44e491ba04ec7e401f55349bdff2116ff4f18ffaee6e0; exit=0; EXPECT=matched; output-sha256=489a4f378cdc5daaec0d205d686c0e1a1179cd45d4d6548e6e544834e2c95157; output-bytes=421; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=58b3c64490f0/49 entries

- [x] G5: 짝 없는 fieldEnd 는 여전히 거부된다 — 어느 문단에도 begin 없음·end 가 begin 보다 앞 문단·같은 문단에서 end 가 앞·이미 닫힌 begin 에 end 재등장 4 경우와 e81c3b2 의 기존 거부 시험. 닫히지 않은 begin 은 열리되 저장이 거부된다
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph -- unmatched_field_end unclosed_field_begin && cargo test --offline --test hwpx_roundtrip_integration c8_native_field_binary_defaults_and_invalid_references_are_explicit
  EXPECT: /test result: ok\. 2 passed; 0 failed[\s\S]*test result: ok\. 1 passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=bdeccc6bf1e92276dfe6298f7e87968d30143f231ac780d806aabed341324ea1; exit=0; EXPECT=matched; output-sha256=f855e3def37bdef4d35c874007640d38f2102fe385f89df21c75515e5fa08411; output-bytes=811; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=58b3c64490f0/49 entries

- [x] G6: 6 양식(G·T1·T2·T5·T6·T7) 조건 1 판정에서 base 82e3da33 보다 나빠진 양식 0, 열기 회귀 0, 대조 — base 판정이 pin lane 의 82e3da33 판정을 6/6 재현
  CHECK: cd /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c2-ac3 && python3 -B cond1_compare.py --base 82e3da33f241558e09c3a21f24469d815b4aa758 --head e8722d6e6a743f4881fb2da3d85c0f1b568c2595
  EXPECT: /pin_lane_82e3da33_verdict_match=6\/6[\s\S]*RHWP_OPEN forms=6 base_open=5 head_open=6 open_regress=0[\s\S]*RHWP_COND1 forms=6 base_pass=1 head_pass=2 regress=0/
  EVIDENCE: automatic-evidence=v1; definition-sha256=9eea8db9eb0b02ec782e100c814fd634899711b146356c17e18dbef5a536a623; exit=0; EXPECT=matched; output-sha256=246f0b7db9df9a5ebe9b68cbcc0cafa2a8b8ea48c630b8187f8ada3698dbca17; output-bytes=731; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=58b3c64490f0/49 entries
  NOTE: 실행 기록 — 필드 짝 커밋 a969ffa8 대상 `20261116T105622Z`·`20261116T105842Z`(head_pass=1, T5 는 그림 저장 결함으로 fail) · 그림 커밋 0e9274b2 대상 `20261116T110600Z`(`$R/ac3-run-2.log`) — T5 fail→pass, T2 pass→pass, G/T1/T6/T7 fail→fail.
  NOTE: 대조 출처 = `/Users/ken/.multiagent/reports/v380-rhwp-l2-pin/work/20261116T031906Z/summary.json`(head=82e3da33f2…, 도구가 `pin['head'] == --base` 를 단언). 0e9274b2 에서 T5 는 열기·저장·재열기·재저장·최소 편집 전 과정을 통과한다.

  NOTE: C1 run 20261116T120042Z tests exact product checkpoint fc35a04d48a557a12729be69df998d45d72a2d95. Comparator and cond1_core.py are byte-identical copies in c1-ac3; prior snapshots and report tools were preserved. G/T1/T6/T7 remain pre-existing failures; T2 pass and T5 pass.

- [x] G7: rhwp `cargo test` 전체 통과(개수)
  CHECK: bash -o pipefail -c 'cargo test --offline 2>&1 | awk "/^test result:/{p+=\$4; f+=\$6; i+=\$8; s++} END{print \"suites=\"s\" passed=\"p\" failed=\"f\" ignored=\"i}"'
  EXPECT: suites=19 passed=1275 failed=0 ignored=7
  EVIDENCE: automatic-evidence=v1; definition-sha256=698f463e1abdd929ff4747ccefa5928d20bb9886d32066e0bbff88729c6299ee; exit=0; EXPECT=matched; output-sha256=b916136988bcddd65902700a91e12f6a92e3bc4ea86b74a5edd38ac33e72d241; output-bytes=41; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=58b3c64490f0/49 entries

- [x] G8: C2 MUST 범위 — initial/C1 기능 파일과 private Paragraph API를 써야 하는 기계적 caller 67개만 바뀐다
  CHECK: git diff --name-only 82e3da33f241558e09c3a21f24469d815b4aa758..HEAD | sort | tr '\n' ' '
  EXPECT: examples/hwpx_dump_text.rs gates/RHWP_FIELDEND_FIX_GATES.md src/document_core/commands/clipboard.rs src/document_core/commands/document.rs src/document_core/commands/footnote_ops.rs src/document_core/commands/formatting.rs src/document_core/commands/header_footer_ops.rs src/document_core/commands/html_import.rs src/document_core/commands/object_ops.rs src/document_core/commands/table_ops.rs src/document_core/commands/text_editing.rs src/document_core/helpers.rs src/document_core/html_table_import.rs src/document_core/queries/bookmark_query.rs src/document_core/queries/cursor_nav.rs src/document_core/queries/cursor_rect.rs src/document_core/queries/doc_tree_nav.rs src/document_core/queries/field_query.rs src/document_core/queries/rendering.rs src/document_core/queries/search_query.rs src/main.rs src/model/control.rs src/model/image.rs src/model/paragraph.rs src/model/paragraph/text.rs src/model/table.rs src/parser/body_text.rs src/parser/body_text/tests.rs src/parser/control/tests.rs src/parser/hwpx/section.rs src/renderer/composer.rs src/renderer/composer/line_breaking.rs src/renderer/composer/lineseg_compare_tests.rs src/renderer/composer/re_sample_gen.rs src/renderer/composer/tests.rs src/renderer/height_measurer.rs src/renderer/layout.rs src/renderer/layout/paragraph_layout.rs src/renderer/layout/shape_layout.rs src/renderer/layout/table_layout.rs src/renderer/layout/tests.rs src/renderer/pagination/engine.rs src/renderer/pagination/tests.rs src/renderer/typeset.rs src/serializer/body_text.rs src/serializer/cfb_writer/tests.rs src/serializer/control/tests.rs src/serializer/hwpx/mod.rs src/serializer/hwpx/picture.rs src/serializer/hwpx/section.rs src/serializer/hwpx/shape.rs src/serializer/hwpx/table.rs src/wasm_api.rs src/wasm_api/tests.rs tests/cell_format_reflow.rs tests/cell_reflow_geometry.rs tests/common/vertical_baseline.rs tests/common/vertical_geometry_matrix.rs tests/hwpx_field_cross_paragraph.rs tests/hwpx_header_resave_stability.rs tests/hwpx_imageless_picture.rs tests/hwpx_page_def_roundtrip.rs tests/hwpx_phase1_corpus.rs tests/hwpx_roundtrip_integration.rs tests/hwpx_save_fidelity_l2.rs tests/hwpx_table_fidelity.rs tests/pagination_square_wrap.rs
  EVIDENCE: automatic-evidence=v1; definition-sha256=aad4e441545d4719f776edbfc05eff85e577aaa53ef0e843e5bfde5c104ca890; exit=0; EXPECT=matched; output-sha256=b560d0cf2c8afb9b8c97adffa182e9a1259ef7361cfa61eed15e73d646108fb9; output-bytes=2201; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=58b3c64490f0/49 entries

NOTE: G8 C1 범위 — HwpxField::span 을 삭제하고 Paragraph::hwpx_field_markers 가 교차 문단 경계를 소유한다. 직접 field_ranges 를 옮기는 document.rs·field_query.rs 도 같은 shift helper 에 연결했다. image.rs 와 wasm_api/tests.rs 의 두 Picture 초기화는 Root msg_dd0ab753ae6e 승인 범위다.

- [x] G9: 기능 변경 owner 파일에 clippy 경고 0; 기계적 caller의 기존 clippy 경고는 이전 C1 출력과 대사해 carry
  CHECK: cargo clippy --offline --lib --tests > /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c2-clippy-gate.log 2>&1 && awk '/^[[:space:]]*--> (src\/model\/(control|image|paragraph|paragraph\/text)\.rs|src\/parser\/(hwpx\/section|body_text)\.rs|src\/serializer\/hwpx\/(table|section|shape|picture)\.rs|src\/document_core\/(commands\/(document|table_ops)|queries\/field_query)\.rs|tests\/hwpx_(field_cross_paragraph|imageless_picture)\.rs)/{n++} END{print "CLIPPY_CHANGED_CODE_WARNINGS="n+0; exit(n>0)}' /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c2-clippy-gate.log
  EXPECT: CLIPPY_CHANGED_CODE_WARNINGS=0
  EVIDENCE: automatic-evidence=v1; definition-sha256=58a381c67c1003c45f2da30405c57406d09bc1504602aceb16ceac9f24073401; exit=0; EXPECT=matched; output-sha256=1756e7f07aced73d1dc321aac152c855522ee6984420c5a3a1fb4034c579d8f2; output-bytes=31; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=58b3c64490f0/49 entries

- [x] G10: 그림 없는 그림 개체(`binaryItemIDRef=""`)가 같은 빈 참조로 저장되고(새 bin 항목·다른 그림 참조 없음) 두 번째 저장이 바이트 같다 · 비어 있지 않은 미등록 참조(`image7`·`foo`·`image0`)는 여전히 저장 거부 · 원본 T5 가 파서·DocumentCore 두 경로로 저장되고 저장본이 다시 열린다(probe 저장→재열기 포함)
  CHECK: cargo test --offline --test hwpx_imageless_picture && /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c2-ac3/target-probe-head/debug/rhwp-gate-probe-head save /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/corpus/T5.hwpx /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c2-t5-raw-saved.hwpx && /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c2-ac3/target-probe-head/debug/rhwp-gate-probe-head ir /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c2-t5-raw-saved.hwpx >/dev/null && echo T5_RAW_SAVE_REOPEN_OK
  EXPECT: /test result: ok\. 4 passed; 0 failed[\s\S]*T5_RAW_SAVE_REOPEN_OK/
  EVIDENCE: automatic-evidence=v1; definition-sha256=4e6f971e3373123bd3485d1484bf8f6d144578ae9f5f08272c747ee57c989985; exit=0; EXPECT=matched; output-sha256=2cef8ea8def822364da701ea3b41505a3aa9e449ce459a5057818531bcae96aa; output-bytes=635; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=58b3c64490f0/49 entries
  NOTE: 빨강 대조 — 같은 시험 파일을 직전 커밋 767d0cfc 트리에 넣어 돌리면 `test result: FAILED. 0 passed; 3 failed`(`$R/picture-redcheck-767d0cf.txt`). 필드 짝 보존은 G3 이 원본 T5 로 잰다.

- [x] G11: RHWPFIX-C1 — private text/offsets enforce the owner; both formula writers refuse removed endpoints; one deletion boundary rule preserves insert/undo/redo; insert/delete/split/merge and second-save bytes stay correct
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph c1_ && cargo test --offline --doc model::paragraph::Paragraph
  EXPECT: /test result: ok\. 9 passed; 0 failed[\s\S]*test result: ok\. 2 passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=5932cec1e0479e0fb9dd3df41e4fdd33b04aa3d7fa28430621c41e8fee1cee19; exit=0; EXPECT=matched; output-sha256=ceb54d5a7050b0a6f0324d6ab6e4393cb7280c6c152fc907fc7693f8c52c42d3; output-bytes=1238; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=58b3c64490f0/49 entries

- [x] G12: RHWPFIX-C2 — cross-paragraph acceptance equals serialized lists including ancestor reachability; every list refuses unclosed begins at open or save
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph c2_
  EXPECT: /test result: ok\. 2 passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=9ad5a6424c51e2fefc55897151af612535c8434ae45fea817419f587e6291610; exit=0; EXPECT=matched; output-sha256=d753c88e4ef1384c926c04c8d57fd5da5cc8cddc446eae2ce4d99aebfa2f7369; output-bytes=430; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=58b3c64490f0/49 entries

- [x] G13: RHWPFIX-C3 — missing picture reference refuses save; explicitly empty and registered numbered references keep existing round trips; unregistered references refuse
  CHECK: cargo test --offline --test hwpx_imageless_picture
  EXPECT: /test result: ok\. 4 passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=66fa991bab64aa3c2bc6652888c5dd73f21c58d18e46af72efda2d480841edda; exit=0; EXPECT=matched; output-sha256=e113ca279c82f532eb021c53f83213f900242c6c9820bf1f309f902100002bd5; output-bytes=577; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=58b3c64490f0/49 entries

NOTE: C2 product checkpoint e8722d6e6a743f4881fb2da3d85c0f1b568c2595, reviewed delta 71178f4a9a3950c5852754e3aea50a9931920b86..e8722d6e6a743f4881fb2da3d85c0f1b568c2595. G2 carries the immutable 82e3da33 source/probe/fixture and prior automatic evidence; this delta changes none of its inputs. The private owner API reaches 61 Rust files, so affected field/parse/round-trip/class guards, AC3 and the suite are rechecked. No cold start, mutation census or CI is added in this correction.

NOTE: C2 RHWPFIX-C1 cause = text writable outside the owner. Private text/char_offsets plus immutable getters remove that bypass; new code is src/model/paragraph/text.rs, with the one marker rule shared by insertion, deletion and replacement. Both formula result writers call replace_text. Guide cleanup/field range replacement use delete_text_at/replace_text_range. The former delete test now straddles the endpoint, because deleting exactly up to it preserves it under the explicit Root rule. Sweeps and red evidence: /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c2-text-offset-sweep.log, c2-red-71178f4a9.log (five new failures), c2-privacy-red-71178f4a9.log (raw writes compiled on baseline), C2_RETURN.md.

NOTE: C2 source tests passed on e8722d6e6a743f4881fb2da3d85c0f1b568c2595: 19 suites, 1275 passed, 0 failed, 7 ignored, including 2 compile-fail privacy tests; c2-cargo-test-final.log. This is executor evidence; R3 and Root landing review remain required.
