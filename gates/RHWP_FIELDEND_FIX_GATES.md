# Gates: rhwp — 문단을 건너 끝나는 HWPX 필드 열기·왕복 (RHWP-FIELDEND-CROSS-PARAGRAPH-V380@r1)

- base `82e3da33f241558e09c3a21f24469d815b4aa758` · 필드 짝 커밋 `a969ffa8eac5bfcdd7c8c14e034072cca7589372` · 그림 없는 그림 커밋 `0e9274b24d7eec705001d2b3596afae901dbf6d2`(Root 결정 B 2026-11-16 · msg_b31f1f933598)
- 초기 측정(이력) 폴더 `R=/Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix` — pin lane 도구 사본(`cond1_compare.py` 경로·대조 출처만 바꿈, `scripts/cond1_core.py` 는 pin lane 과 바이트 같음 · sha256 `7e0607a5…`), 코퍼스 6 양식(`corpus_sha256.txt` 대사는 도구가 v374 목록으로 한다), probe 두 벌(`src-base`=82e3da33 · `src-head`=0e9274b2, 도구가 git blob 전수 대사).
- 공통 — 아래 CHECK 는 워크트리 `/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix` 에서 돈다.

- C1 product checkpoint `fc35a04d48a557a12729be69df998d45d72a2d95`. G6/G7/G9 carry the final-source runs across subsequent ledger-only commits; the final checker run also repeated them because free-form evidence was not treated as current automatic evidence. Class guards are rerun after the correction. New source scope includes Paragraph edit owners and Root approval msg_dd0ab753ae6e for image.rs and mechanical Picture initializers.

- [x] G1: head 에서 T5 원본(sha256 a0bd6914…)이 파싱 오류 0 으로 열린다(파서·DocumentCore 두 경로, probe `ir` 도 rc 0)
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph t5_opens_through_parser_and_document_core && /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c6-ac3/target-probe-head/debug/rhwp-gate-probe-head ir /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/corpus/T5.hwpx >/dev/null && echo head_ir_rc=0
  EXPECT: /test result: ok\. 1 passed; 0 failed[\s\S]*head_ir_rc=0/
  EVIDENCE: automatic-evidence=v1; definition-sha256=e6b2b3b0be9f488431481c859ce9d7c7e4990a6dce33e23dab76cd2698a1d650; exit=0; EXPECT=matched; output-sha256=60fab794cce421d69c123cae81eb921f3de0515d61a4e82eb4f1ea1723336506; output-bytes=414; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix/gates; path=5f6aef6c939b/48 entries

- [x] G2: 같은 열기 검사가 base 82e3da33 에서는 실패한다(대조 · 출력 보존 `$R/g2-base-t5-open.txt`)
  CHECK: R=/Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix; $R/target-probe-base/debug/rhwp-gate-probe-base ir $R/corpus/T5.hwpx >/dev/null 2>$R/g2-base-t5-open.txt; echo "base_ir_rc=$?"; cat $R/g2-base-t5-open.txt
  EXPECT: /base_ir_rc=1[\s\S]*fieldEnd beginIDRef에 대응하는 fieldBegin 없음/
  EVIDENCE: automatic-evidence=v1; definition-sha256=5136b26de3e3adc6bde3353bf7ec5b67aa34411b1275eb780851f8938317d4ce; exit=0; EXPECT=matched; output-sha256=beb221437d01783ffb5b5731c48693806e3892a63924d63bb6f7c15ef18adc43; output-bytes=177; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=4f045aa37ddf/49 entries
  NOTE: 새 시험 3 건(T5 왕복·본문·셀)도 base 소스에서 같은 오류로 빨강이었다(구현 전 실행, `t5…`·`body…`·`cell…` FAILED · 짝 없는 end 거부·닫히지 않은 begin 시험 2 건은 base 에서도 초록).

- [x] G3: 원본 T5(수정 없음) 저장→재열기→재저장에서 필드 begin/end 한 쌍이 같은 문단 위치·같은 속성·같은 내용으로 남고, 두 번째 저장 section XML 이 첫 저장과 바이트 같다
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph t5_cross_paragraph_field_round_trips
  EXPECT: test result: ok. 1 passed; 0 failed
  EVIDENCE: automatic-evidence=v1; definition-sha256=8faf6b09f5707fbe29c5abb9344fad58e06ceac2c4d69d5e3ede70a3346f267d; exit=0; EXPECT=matched; output-sha256=bc341703150ce1a1d28ee9252286f210194aeb114cb0168779bb1dbd89ca5e8d; output-bytes=396; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix/gates; path=5f6aef6c939b/48 entries

- [x] G4: 문단을 건너는 필드가 본문 목록·표 셀 목록 모두에서 열리고, 저장→재열기→재저장에서 경계·글자 순서가 원본과 같다(시작 문단은 이 필드의 field_ranges 를 갖지 않는다)
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph spanning_paragraphs
  EXPECT: test result: ok. 2 passed; 0 failed
  EVIDENCE: automatic-evidence=v1; definition-sha256=4cd87ef1f000735faac44e491ba04ec7e401f55349bdff2116ff4f18ffaee6e0; exit=0; EXPECT=matched; output-sha256=335210f3e9705fcf3b39fafb341fa7fec666347f22d19ec1538f6b675a458e60; output-bytes=468; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix/gates; path=5f6aef6c939b/48 entries

- [x] G5: 짝 없는 fieldEnd 는 여전히 거부된다 — 어느 문단에도 begin 없음·end 가 begin 보다 앞 문단·같은 문단에서 end 가 앞·이미 닫힌 begin 에 end 재등장 4 경우와 e81c3b2 의 기존 거부 시험. 닫히지 않은 begin 은 열리되 저장이 거부된다
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph -- unmatched_field_end unclosed_field_begin && cargo test --offline --test hwpx_roundtrip_integration c8_native_field_binary_defaults_and_invalid_references_are_explicit
  EXPECT: /test result: ok\. 2 passed; 0 failed[\s\S]*test result: ok\. 1 passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=bdeccc6bf1e92276dfe6298f7e87968d30143f231ac780d806aabed341324ea1; exit=0; EXPECT=matched; output-sha256=c43ce8faf255050c546e64b8e9efd731f07bc50a79d1fcfe5bc53888ec622200; output-bytes=905; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix/gates; path=5f6aef6c939b/48 entries

- [x] G6: 6 양식(G·T1·T2·T5·T6·T7) 조건 1 판정에서 base 82e3da33 보다 나빠진 양식 0, 열기 회귀 0, 대조 — base 판정이 pin lane 의 82e3da33 판정을 6/6 재현
  CHECK: python3 /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c6-ac3/cond1_compare.py --base 82e3da33f241558e09c3a21f24469d815b4aa758 --head fcf54778fca9916800017278fabd0dcf3f812517 > /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c6-ac3.log 2>&1 && cat /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c6-ac3.log
  EXPECT: /pin_lane_82e3da33_verdict_match=6\/6[\s\S]*RHWP_OPEN forms=6 base_open=5 head_open=6 open_regress=0[\s\S]*RHWP_COND1 forms=6 base_pass=1 head_pass=2 regress=0/
  EVIDENCE: automatic-evidence=v1; definition-sha256=7ddedb5ae11b1bbe1cd2f78d769eba507127f89f3589e35a20ff2ffaeeead3a6; exit=0; EXPECT=matched; output-sha256=6b06f4a97022fba46011cb8c9e8f1a3b0d30b3fb2e79473d8d2cc09f7660f4e2; output-bytes=731; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix/gates; path=5f6aef6c939b/48 entries
  NOTE: 대조 출처 = `/Users/ken/.multiagent/reports/v380-rhwp-l2-pin/work/20261116T031906Z/summary.json`(head=82e3da33f2…, 도구가 `pin['head'] == --base` 를 단언). C6 product checkpoint `fcf54778fca9916800017278fabd0dcf3f812517` 에서 T2·T5 가 통과하고 G/T1/T6/T7 은 기존 상태를 유지한다.

- [x] G7: full cargo suite on final C6 product source
  CHECK: cargo test --offline > /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c6-cargo-test-final.log 2>&1 && cat /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c6-cargo-test-final.log && echo CARGO_FULL_OK
  EXPECT: /test result: ok\.[\s\S]*CARGO_FULL_OK/
  EVIDENCE: automatic-evidence=v1; definition-sha256=1ba7f234efd98d53f21e01da200593064def16c55df3b26e5e589ca7a1cc621a; exit=0; EXPECT=matched; output-sha256=67b4050e997d039059844680ca736bef43b67d0d0d4b057463166a7cfb8c54b8; output-bytes=96983; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix/gates; path=5f6aef6c939b/48 entries

- [x] G8: C6 cumulative MUST scope from base 82e3da33 through C6
  CHECK: git diff --name-only 82e3da33f241558e09c3a21f24469d815b4aa758..HEAD | sort | tr '\n' ' '
  EXPECT: Cargo.toml examples/hwpx_dump_text.rs gates/RHWP_FIELDEND_FIX_GATES.md rhwp-studio/src/core/wasm-bridge.ts rhwp-studio/src/engine/command.ts rhwp-studio/src/engine/history.ts rhwp-studio/src/engine/input-handler-keyboard.ts rhwp-studio/src/engine/input-handler-mouse.ts rhwp-studio/src/engine/input-handler-picture.ts rhwp-studio/src/engine/input-handler-table.ts rhwp-studio/src/engine/input-handler-text.ts rhwp-studio/src/engine/input-handler.ts rhwp-studio/src/hwpctl/index.ts rhwp-studio/src/ui/find-dialog.ts rhwp-studio/tests/field-undo-command.test.ts src/document_core/commands/clipboard.rs src/document_core/commands/document.rs src/document_core/commands/footnote_ops.rs src/document_core/commands/formatting.rs src/document_core/commands/header_footer_ops.rs src/document_core/commands/html_import.rs src/document_core/commands/object_ops.rs src/document_core/commands/table_ops.rs src/document_core/commands/text_editing.rs src/document_core/helpers.rs src/document_core/html_table_import.rs src/document_core/mod.rs src/document_core/queries/bookmark_query.rs src/document_core/queries/cursor_nav.rs src/document_core/queries/cursor_rect.rs src/document_core/queries/doc_tree_nav.rs src/document_core/queries/field_query.rs src/document_core/queries/rendering.rs src/document_core/queries/search_query.rs src/error.rs src/main.rs src/model/control.rs src/model/image.rs src/model/paragraph.rs src/model/paragraph/tests.rs src/model/paragraph/text.rs src/model/table.rs src/model/table/tests.rs src/parser/body_text.rs src/parser/body_text/tests.rs src/parser/control.rs src/parser/control/tests.rs src/parser/hwpx/section.rs src/renderer/composer.rs src/renderer/composer/line_breaking.rs src/renderer/composer/lineseg_compare_tests.rs src/renderer/composer/re_sample_gen.rs src/renderer/composer/tests.rs src/renderer/height_measurer.rs src/renderer/layout.rs src/renderer/layout/paragraph_layout.rs src/renderer/layout/shape_layout.rs src/renderer/layout/table_layout.rs src/renderer/layout/tests.rs src/renderer/pagination/engine.rs src/renderer/pagination/tests.rs src/renderer/typeset.rs src/serializer/body_text.rs src/serializer/cfb_writer/tests.rs src/serializer/control/tests.rs src/serializer/hwpx/mod.rs src/serializer/hwpx/picture.rs src/serializer/hwpx/section.rs src/serializer/hwpx/shape.rs src/serializer/hwpx/table.rs src/wasm_api.rs src/wasm_api/tests.rs tests/cell_format_reflow.rs tests/cell_reflow_geometry.rs tests/cell_vertical_baseline.rs tests/common/reflow_geometry.rs tests/common/vertical_baseline.rs tests/common/vertical_geometry_fixture.rs tests/common/vertical_geometry_matrix.rs tests/hwpx_field_cross_paragraph.rs tests/hwpx_header_resave_stability.rs tests/hwpx_imageless_picture.rs tests/hwpx_page_def_roundtrip.rs tests/hwpx_phase1_corpus.rs tests/hwpx_roundtrip_integration.rs tests/hwpx_save_fidelity_l2.rs tests/hwpx_table_fidelity.rs tests/pagination_square_wrap.rs
  EVIDENCE: automatic-evidence=v1; definition-sha256=10adaecbb6c33401435c782de8e16edb6b034dc8cf4454d8302255e8bd441be2; exit=0; EXPECT=matched; output-sha256=518f7835a0041a1c26876e5ad59a485532790bf79f78d4a2e26cf03edd54126f; output-bytes=2921; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix/gates; path=5f6aef6c939b/48 entries

- [x] G9: Clippy diagnostics match the prior baseline or are removed; changed code adds none
  CHECK: bun /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c6-clippy-guard.mjs
  EXPECT: CLIPPY_BASELINE_WARNINGS=46 HEAD_WARNINGS=36 CLIPPY_CHANGED_CODE_WARNINGS=0
  EVIDENCE: automatic-evidence=v1; definition-sha256=79a48e04bfb7835268534310c5c54d316bd44ced144fddb1b49ee16453a94726; exit=0; EXPECT=matched; output-sha256=5a66e4665cb44f4ea60e88b88d30ec711c1e665e60e59772862c626b0042eb1a; output-bytes=76; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix/gates; path=5f6aef6c939b/48 entries

- [x] G10: 그림 없는 그림 개체(`binaryItemIDRef=""`)가 같은 빈 참조로 저장되고(새 bin 항목·다른 그림 참조 없음) 두 번째 저장이 바이트 같다 · 비어 있지 않은 미등록 참조(`image7`·`foo`·`image0`)는 여전히 저장 거부 · 원본 T5 가 파서·DocumentCore 두 경로로 저장되고 저장본이 다시 열린다(probe 저장→재열기 포함)
  CHECK: cargo test --offline --test hwpx_imageless_picture && /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c6-ac3/target-probe-head/debug/rhwp-gate-probe-head save /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/corpus/T5.hwpx /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c6-t5-raw-saved.hwpx && /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c6-ac3/target-probe-head/debug/rhwp-gate-probe-head ir /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c6-t5-raw-saved.hwpx >/dev/null && /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c6-ac3/target-probe-head/debug/rhwp-gate-probe-head save /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c6-t5-raw-saved.hwpx /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c6-t5-raw-resaved.hwpx && /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c6-ac3/target-probe-head/debug/rhwp-gate-probe-head ir /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c6-t5-raw-resaved.hwpx >/dev/null && echo T5_RAW_SAVE_REOPEN_RESAVE_OK
  EXPECT: /test result: ok\. 4 passed; 0 failed[\s\S]*T5_RAW_SAVE_REOPEN_RESAVE_OK/
  EVIDENCE: automatic-evidence=v1; definition-sha256=e9b6b030a39e1e0dc45d7b20461110e74917bf97dce4aa4f454745000eaf81b1; exit=0; EXPECT=matched; output-sha256=8972bfe138ec0eed15913d29fb1dc6e3be99ae3c1751e8132743d4c733cab4a6; output-bytes=725; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix/gates; path=5f6aef6c939b/48 entries

- [x] G11: RHWPFIX-C1 D1-D13: single state-label inverse gate, private model API, half-open control membership and snapshot history suspension through native and actual WASM APIs
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph c1_ && cargo test --offline --doc model::paragraph::Paragraph && cargo test --offline --lib test_task231_field_survives && cargo build --offline --target wasm32-unknown-unknown --lib && cargo run --offline --manifest-path /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c3-bindgen/Cargo.toml && ./rhwp-studio/node_modules/.bin/tsc --noEmit -p rhwp-studio/tsconfig.json && echo STUDIO_TSC_OK && bun test ./rhwp-studio/tests/field-undo-command.test.ts
  EXPECT: /test result: ok\. 41 passed; 0 failed[\s\S]*test result: ok\. 6 passed; 0 failed[\s\S]*test result: ok\. 2 passed; 0 failed[\s\S]*STUDIO_TSC_OK[\s\S]*MUTATION_API_BOUNDARY owners=3 public_mutators=0 raw_write_refused=true[\s\S]*\b52 pass\b[\s\S]*\b0 fail\b/
  CWD: ..
  EVIDENCE: automatic-evidence=v1; definition-sha256=10e1f86ba42410ac5d2cbdda741000f66ae94a6b373d675ea7ead5ac8c852311; exit=0; EXPECT=matched; output-sha256=7e1949678ec67c89ebbec1f473bf79d716ac82647e20a153553a194679c1e2db; output-bytes=46109; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=5f6aef6c939b/48 entries

- [x] G12: RHWPFIX-C2 — cross-paragraph acceptance equals serialized lists including ancestor reachability; every list refuses unclosed begins at open or save
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph c2_
  EXPECT: /test result: ok\. 2 passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=9ad5a6424c51e2fefc55897151af612535c8434ae45fea817419f587e6291610; exit=0; EXPECT=matched; output-sha256=8f6fbe4e7f038b49599f607eb3b2cec485db2edcaa2e34c3856a12baa12cd0a3; output-bytes=1714; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix/gates; path=5f6aef6c939b/48 entries

- [x] G13: RHWPFIX-C3 — missing picture reference refuses save; explicitly empty and registered numbered references keep existing round trips; unregistered references refuse
  CHECK: cargo test --offline --test hwpx_imageless_picture
  EXPECT: /test result: ok\. 4 passed; 0 failed/
  EVIDENCE: automatic-evidence=v1; definition-sha256=66fa991bab64aa3c2bc6652888c5dd73f21c58d18e46af72efda2d480841edda; exit=0; EXPECT=matched; output-sha256=28eb61cf2af99de41aea886b22d11deab8c8fcc92af691bc904d3f7870a3b047; output-bytes=698; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix/gates; path=5f6aef6c939b/48 entries

- [x] G14: RHWPFIX-C1 snapshot operations reject nested history records at runtime and keep one undo entry with zero retained inverses after clear (reporter-portable EXPECT)
  CHECK: bun test ./rhwp-studio/tests/field-undo-command.test.ts -t "snapshot rejects|selection paste has|IME in a selection|object cut snapshot|table drag drop|Studio commands release|selection route|IME cancellation|iOS provisional"
  EXPECT: /STUDIO_COMMAND_LIFECYCLES=18 live-token delta=0[\s\S]*\b10 pass\b[\s\S]*\b0 fail\b[\s\S]*Ran 10 tests across 1 file\./
  CWD: ..
  EVIDENCE: automatic-evidence=v1; definition-sha256=a3a0ff9a75ecd31897772d0327ece0184d08ed6a96517772fc941c81b41d4b51; exit=0; EXPECT=matched; output-sha256=9a3792fe757fe5c167eff03f2a98f6c684b1795b730d69fc6e068450edc4378f; output-bytes=4947; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=5f6aef6c939b/48 entries

- [x] G15: Every R6 witness listed in D13 fails at 8b233638e and passes at HEAD natively and through WasmBridge
  CHECK: node /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c6-g15-witnesses.mjs
  EXPECT: G15_WITNESSES total=24 native=7 wasm=17 baseline_8b233638e_failed=24/24 head_passed=24/24
  CWD: ..
  EVIDENCE: automatic-evidence=v1; definition-sha256=6dc6e75bac6e3e0c1cb594ec51a6195750aabca6d2a9a214df509246d6e8eb60; exit=0; EXPECT=matched; output-sha256=85b6785b7cb9560f6b8e19b9330ba89748727c7f66fa82241986ea9139d93695; output-bytes=90; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=5f6aef6c939b/48 entries

- [x] G16: Deterministic mixed-sequence property test runs seed range 0..63 with 0 invariant violations at HEAD and fails at 8b233638e on at least one seed
  CHECK: node /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c6-g16-property.mjs
  EXPECT: /G16_PROPERTY seed_range=0\.\.63 seeds=64 steps=1536 baseline_8b233638e_failed_seeds=[1-9][0-9]*\/64 first_baseline_failure_seed=[0-9]+ head_violations=0/
  CWD: ..
  EVIDENCE: automatic-evidence=v1; definition-sha256=352eb12238166c4802033328a2b48156c174bb6376ef330ada36c9a20f2e06bd; exit=0; EXPECT=matched; output-sha256=0a3cf63740b9a43e1fc034f23b18ad71237b014bd53e36ef10d9035afbfb01ea; output-bytes=136; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=5f6aef6c939b/48 entries

- [x] G17: Source-level visibility and call-graph guard fails if a second inverse-acceptance path or a second control-membership function is added
  CHECK: cargo test --offline --test hwpx_field_cross_paragraph g17_rust_single_inverse_authority_and_single_membership_owner_guard -- --nocapture && bun test ./rhwp-studio/tests/field-undo-command.test.ts -t "G17 TypeScript AST guard"
  EXPECT: /G17_RUST_GUARD inverse_authority=1 state_writers=2 anchor_consumers=2 copy_delete_delegated=true[\s\S]*G17_TS_GUARD inverse_callers=1 private_stacks=2 dual_label_entry=true[\s\S]*\b1 pass\b[\s\S]*\b0 fail\b/
  CWD: ..
  EVIDENCE: automatic-evidence=v1; definition-sha256=2bd8cde1ce90f7d776defc27bd045932eea9c532aa0f450ec6a68dd7cb0dd13b; exit=0; EXPECT=matched; output-sha256=443f5d8d6d034a3d6c9816fb3e07afce65e3b64c1f878832e49950bdf802320c; output-bytes=804; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=5f6aef6c939b/48 entries

- [x] G18: Full cargo test passes with 0 failed, Studio tests pass with 0 failed, binary HWP is 156/156 on both save paths, and six forms show 0 regressions with control at 6/6
  CHECK: node /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c6-g18-suite.mjs
  EXPECT: /G18_FULL cargo_passed=1312 cargo_failed=0 studio_passed=130 studio_failed=0 binary_raw=156\/156 binary_rebuilt=156\/156 forms_control=6\/6 forms_regress=0/
  CWD: ..
  EVIDENCE: automatic-evidence=v1; definition-sha256=49a8c9abba6aae0d3455bccedd560cce062df2acb17fb04cc5024a9541813748; exit=0; EXPECT=matched; output-sha256=3922e8329c4927fc92dbf890b0b5392df7b3420701834d94538d089b371d6388; output-bytes=152; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=5f6aef6c939b/48 entries

- [x] G19: Scope diff 8b233638e..HEAD touches only section 3.1 MUST paths, WASM bindings are additive, and Cargo.lock is unchanged
  CHECK: node /Users/ken/.multiagent/reports/v380-rhwp-fieldend-fix/c6-g19-scope.mjs
  EXPECT: /G19_SCOPE files=(?:21|22) disallowed=0 cargo_lock_sha256=2fb214553428c723a593345de02ab7ec06b3da921bc843cc64c9360abfb69a7c wasm_sigs=281->291 additive=true/
  CWD: ..
  EVIDENCE: automatic-evidence=v1; definition-sha256=742b6cefe4a6395b2ea004054a96c0c4dcd309458f62152c4af51aadd9ee292d; exit=0; EXPECT=matched; output-sha256=d135c3317ec28160e5f34df718628ac9a304d775f6ae32c2e615b04d6eaf29c5; output-bytes=1491; shell=/bin/sh; cwd=/Users/ken/Projects/_wt-rhwp/v380-fieldend-fix; path=5f6aef6c939b/48 entries

NOTE: C6 product checkpoint is `fcf54778fca9916800017278fabd0dcf3f812517`; reviewed head from C5 is `8b233638eeed2ac234a1b5dba22f9a3a303f25e0`; immutable base is `82e3da33f241558e09c3a21f24469d815b4aa758`. G2 retains its original immutable-base automatic evidence because none of C6 touches `82e3da33`, `target-probe-base`, or `corpus/T5.hwpx`. All other 18 gates (`G1`, `G3`-`G19`) are freshly executed at C6.
NOTE: C6 RHWPFIX-C1 closure: D11 replaces parallel text/snapshot/command inverse checks and snapshot relabelling (`rebase_edit_revision`) with one `DocumentMutation` content-state label (`epoch:state`) and one gate acceptance method (`begin_inverse`/`finish_inverse`), and every `CommandHistory` entry stores `{undoLabel, redoLabel}` while truncating refused stack suffixes to preserve older applicable entries. D12 replaces parallel copy/delete control logic with one private `Paragraph::control_in_selection` (`start <= (pi, anchor) < end`) called by both `clipboard_fragment` and `delete_selection_range`. D13 adds all 24 native+WASM R6 witnesses (`G15`), the 64-seed x 24-step deterministic mixed-sequence property test (`G16`), and the Rust/TypeScript visibility and call-graph single-authority guards (`G17`). P3 makes `G14`'s `EXPECT` independent of Bun's agent-mode reporter. Full details and sweeps: `C6_RETURN.md`.
