# HWPX save fidelity L2 C1 (v353)

Status: PAUSED by Parent STOP-SAVE msg_450f6ddd4d0d at 2026-10-26T08:10:19Z. Source corrections are complete; the parity-only lint fix has not been revalidated. No push, self-review receipt, worker_done or successful mah finish.

Source checkpoints: `5315315566c6fe75826fe05a9d59880aae62948a` implements F1-F4; `c0d6a342c186a3d57c416ddb262783cc7a064c23` changes only the unsigned tab parity expression to the equivalent Clippy-required `is_multiple_of(2)`.

| Finding | Correction | RED at 8027a28 | GREEN before the parity-only lint edit |
| --- | --- | --- | --- |
| F1 | `src/serializer/hwpx/table.rs:471` merges field boundaries and ordinary controls; `src/parser/hwpx/section.rs:2544` and `:2579` count all control starts and completed field ends. Body `section.rs:173` uses the same writer as cell/nested paragraphs. | Both `run_order_c1_*` tests fail. The IR reproduces fieldBegin after table; raw XML covers empty/nested/adjacent fields around tables in body, cell and nested cell paragraphs, across two native saves. | Both pass; the complete L2 suite passes 18/0/2 and G0 integration 57/0/2. |
| F2 | `src/serializer/hwpx/header.rs:616` writes odd tab case values with CHAR; `src/parser/hwpx/header.rs:1165` restores the unit remainder. | `residual_c1_tab_halving_path_survives_two_saves` fails (pos 1 is incorrectly HWPUNIT). | Passes all unsigned test values over two saves. |
| F3 | `src/serializer/hwpx/header.rs:908` shares the signed margin scaling rule at `:940`; `src/parser/hwpx/header.rs:800` decodes the CHAR remainder. Both line-spacing readers accept the writer's BETWEEN_LINES spelling for SpaceOnly. | `residual_c1_signed_halving_paths_survive_two_saves` fails (odd Fixed/Minimum values lose one; SpaceOnly is parsed as Percent). | All five margins, Fixed/SpaceOnly/Minimum spacing, and the unscaled Percent control pass the signed boundary matrix over two saves. |
| F4 | `src/model/document.rs:130` stores raw flags, `src/parser/hwpx/header.rs:156` reads empty and paired elements, and `src/serializer/hwpx/header.rs:1050` emits the stored value. Missing input defaults to the existing flags=0 output. | `residual_c1_track_change_flags_survive_two_native_saves` fails (56 becomes 0). The absent-element control already passes. | Flags 56, 0 and u32::MAX pass two native saves for both element spellings; absence matches explicit-zero output at each save. |

The latest test file is SHA256 `31b58bbaf1a8d7a1426c192aa1113190d801c350124dc4ac89249aef517bd066`. It uses only APIs present at 59c2285, so both negative controls compile the exact test file without adding model-dependent assertions.

- `red-8027a28-final.log`: a git archive of `8027a28e4d1429f11cec0860d11484d1d9e6e3fe`, with the latest test file overlaid and identical Cargo.lock, produces 1 passed / 5 failed. The passing test is the absent-element control.
- `negative-59c2285.log` and the updated parent `negative-control.json`: L2 2 passed / 16 failed / 2 ignored; run-order failures=5, two-save failures=3 including the G3 library test, residual failures=9. The previous base failure behavior is retained.
- `green-c1.log`: all six new tests pass. `green-l2.log`: L2 18 passed / 2 ignored. `green-g0.log`: G0 integration 57 passed / 2 ignored. `green-lane1.log`: lane 1 39 passed / 1 ignored. `green-g3.log`: the existing secPr/colPr form-attribute test passes.
- `g7-full-initial-5315315.log`: clean archive of 5315315, with identical untracked Cargo.lock, full cargo suite 1252 passed / 0 failed / 7 ignored across 17 result groups, including doctests. `g9-wasm-initial-5315315.log`: WASM release Finished. `g8-clippy-initial-5315315.log`: one manual_is_multiple_of lint; no suppression was added.
- The first gate-check run records 10/11 met, only G8 unmet. The Parent sent HOLD during this run; the chain reached clippy and WASM before the pause took effect. No cargo/rustc/clippy process remained afterward. The paused ledger retains the obtained source-5315315 evidence, with G8 pending; it does not claim final-head verification. The later STOP-SAVE forbids any new heavy validation until resumed.

The complete halving-path inventory is TabItem.position; ParaShape.indent, margin_left, margin_right, spacing_before, spacing_after; and ParaShape.line_spacing for Fixed, SpaceOnly/BETWEEN_LINES and Minimum/AT_LEAST. Percent remains unscaled. Even values use HWPUNIT; odd values use floor(value/2) plus CHAR, with exact full values in default branches.

Every signed path is checked at -1001, -1000, -1, 0, 1, 1000, 1001, i32::MIN, i32::MIN+1, i32::MAX-1 and i32::MAX. Tabs are checked at 0, 1, 1000, 1001, 2^31, 2^31+1, u32::MAX-1 and u32::MAX. Negative tab positions are unrepresentable in the current u32 model; the Parent explicitly approved this test interpretation and forbade expanding that model type.

`original-header-observations.json` records actual fixture observations, including case/default nodes: Hancom C6 tab positions range from 468 to 248000, with zero negative or high-bit values. C6 spacing values range from 100 to 230, with no negatives. Negative values occur in indent (minimum -47656, 640 case/default nodes) and left margin (minimum -200, 14 nodes). These observations do not claim Hancom uses the synthetic boundary cases. FACF has no negative/high-bit tab or negative spacing values either.

Parent scope approval `outputs/v353-rhwp-table/RHWP_HWPX_SAVE_L2_CORRECTION_C1_V353_DISPOSITION1.md` allows only the field-order calculation change in parser/section.rs; the original C1 brief separately permits header parsing and the flags-only model field. G10 includes these paths. No original test assertion was weakened and the V318 ledger is untouched.

Executor inspection: no additional finding was identified in the correction diff. The shared paragraph writer covers both callers; field IDs/attributes remain under the existing G0 tests; odd/even negative arithmetic stays within the full signed model range; flags remain typed u32 with no inferred bit meanings. Existing large source/test files are retained because this correction's approved scope excludes a structural split. There is no unsafe code, new dependency or lint/type suppression.

Known limits retained from the approved packet: C6 p195 follows the existing IR position limit; unmodeled drawing metadata and the previously observed overall page-count discrepancy remain outside C1. The source values needed for the reported first-page box fix are preserved, but the six regenerated files still need the Parent's Hancom UI check. Separate Codex re-review is externally dispatched and has not been claimed as complete.

jev=used (all six changed source/test paths, the ledger and Hancom manifest; deterministic pin/exec candidates retained). Packet/disposition paths returned jev=unused(candidate count 0); no Jev candidate was used to exclude deterministic evidence.

INTENT RHWP-HWPX-SAVE-FIDELITY-L2-V353@r1 sha256:d385ca9b6e37df48347dfc71d28bfbc161c1db249304100a064347645b479366

- AC1: met in the approved fixture scope, with the disposed p195 exception; both F1 tests also pass.
- AC2: met by G3 and both existing two-save tests.
- AC3: met by the residual inventory, F1-F4 RED/GREEN and raw flags preservation; Parent visual confirmation remains its separate stage.
- AC4: pending resumption; the prior clean-archive full suite and WASM pass, G8's lint correction is committed but not rerun, and six review files were regenerated from 5315315. Final-head validation, final-head artifact regeneration and separately dispatched re-review remain.
