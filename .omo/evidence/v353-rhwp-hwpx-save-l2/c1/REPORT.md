# HWPX save fidelity L2 C1 (v353)

Correction complete: ledger **11/11**, no deferrals. Validation and artifact producer head: `a6d024f4f0bda4dcbe01bc247dceef003ea865d4`, with src/tests byte-identical to product head `c0d6a342c186a3d57c416ddb262783cc7a064c23`. The final checkpoint adds only evidence/report/ledger changes; its SHA is supplied in worker_done. No push or self-review receipt.

| Finding | Fix | RED at 8027a28 | GREEN on the final source |
| --- | --- | --- | --- |
| F1 | `src/serializer/hwpx/table.rs:471` merges field and ordinary-control events; `src/parser/hwpx/section.rs:2544` and `:2579` count control starts plus completed field ends. Body `src/serializer/hwpx/section.rs:164` calls the same writer as cell/nested paragraphs at `table.rs:330`. | Both `run_order_c1_*` tests fail, including the reviewer's binary-field IR pattern and raw [fieldBegin,fieldEnd,tbl] input. | Both pass, including empty/nested/adjacent fields before/inside/after tables in body, cell and nested-cell owners across two native saves. |
| F2 | `src/serializer/hwpx/header.rs:616` emits odd tab case values with CHAR; `src/parser/hwpx/header.rs:1168` restores the remainder. | `residual_c1_tab_halving_path_survives_two_saves` fails: odd tab 1 is incorrectly marked HWPUNIT; R1 independently reproduced 1001 becoming 1000. | All unsigned values and case/default representations pass two saves. |
| F3 | `src/serializer/hwpx/header.rs:908` reuses the signed margin rule at `:940`; `src/parser/hwpx/header.rs:807` restores CHAR. Both readers accept the emitted BETWEEN_LINES spelling for SpaceOnly. | `residual_c1_signed_halving_paths_survive_two_saves` fails: odd Fixed/Minimum values lose one and SpaceOnly is read as Percent. | Five margins and all three scaled spacing types pass odd/even/negative/i32 endpoints over two saves; Percent remains unchanged. |
| F4 | Raw u32 field at `src/model/document.rs:130`; empty/paired element parsing at `src/parser/hwpx/header.rs:157`; stored-value output at `src/serializer/hwpx/header.rs:1050`. | `residual_c1_track_change_flags_survive_two_native_saves` fails: 56 becomes 0. The absent-element control passes. | Flags 56, 0 and u32::MAX survive two native saves for empty/paired elements; absence matches explicit-zero output at each save. |

Test file SHA256: `31b58bbaf1a8d7a1426c192aa1113190d801c350124dc4ac89249aef517bd066`. It uses only APIs present at 59c2285 so both negative controls compile the exact test file.

| Evidence | Observed result |
| --- | --- |
| `red-8027a28-final.log`: isolated archive of 8027a28 with the latest test file and identical Cargo.lock | 1 passed / 5 failed; the absent-element control passes |
| `negative-59c2285.log` and `../negative-control.json` | L2 2 passed / 16 failed / 2 ignored; run-order failures 5, two-save failures 3 including G3, residual failures 9; G3 library 0 passed / 1 failed |
| `g7-full.log`: clean archive of a6d024f plus identical untracked Cargo.lock | 1252 passed / 0 failed / 7 ignored, 17 result groups including doctests; all six C1 regression names explicitly pass |
| Final full-suite targets | L2 18 passed / 2 ignored; G0 integration 57 passed / 2 ignored; lane 1 39 passed / 1 ignored; G3 library test passes within the full library target |
| `g8-clippy.log` | `cargo clippy -- -D warnings`, exit 0, Finished |
| `g9-wasm.log` | WASM release build, exit 0, Finished |
| `gate-run-final.log` | G4/G7/G8/G9/G10/G11 freshly PASS; ALL MET (11 met), with the declared unaffected carries |
| `hancom-generate-final.log` and `../hancom/MANIFEST.txt` | 6 files regenerated from a6d024f; ZIP/XML and hashes checked; C6 flags 56, FACF flags 0 |

The complete halving-path inventory is **TabItem.position; ParaShape.indent (intent), margin_left (left), margin_right (right), spacing_before (prev), spacing_after (next); ParaShape.line_spacing for Fixed, SpaceOnly/BETWEEN_LINES and Minimum/AT_LEAST**. Percent is unscaled. Even case values use HWPUNIT; odd values use floor(value/2) with CHAR; default branches retain the full original value. The reader computes case*2+CHAR.

Every signed path is tested at -1001, -1000, -1, 0, 1, 1000, 1001, i32::MIN, i32::MIN+1, i32::MAX-1 and i32::MAX. Unsigned tabs are tested at 0, 1, 1000, 1001, 2^31, 2^31+1, u32::MAX-1 and u32::MAX. **Negative tabs are unrepresentable in the current u32 model**, as explicitly approved by the Parent; that model type was not expanded.

Actual fixture observations are in `original-header-observations.json`, counting case/default nodes: C6 tab positions range 468..248000 with zero negatives/high-bit values; spacing ranges 100..230 with no negatives. Negative indent reaches -47656 (640 nodes) and left margin -200 (14 nodes). The synthetic boundary tests do not claim those values occur in Hancom originals.

Scope: original packet and dispositions 1/2 plus the C1 brief and `outputs/v353-rhwp-table/RHWP_HWPX_SAVE_L2_CORRECTION_C1_V353_DISPOSITION1.md`. Parser/section changes only the field-order calculations; DocInfo adds only the flags field. G10 allows the approved parser/header, parser/section and model/document paths. No original test assertion was weakened. V318 ledger, renderer, table geometry/height/width, pageBreak mapping and submodule pins are unchanged.

Executor diff inspection found no additional blocking finding. The paragraph writer is shared by body/cell callers; existing G0 tests preserve field IDs/attributes; signed boundary arithmetic is exercised through parse/model/save; flags are typed raw u32 with default zero and no assigned bit meanings. Existing large modules remain because this correction's narrow scope excludes a structural split. No unsafe code, dependency, type escape hatch or lint suppression was added. This is current-session executor evidence, not independent assurance.

The initial source 5315315 passed full tests/WASM but hit one manual_is_multiple_of lint; c0d6a34 fixed that expression and final validation above passes. Initial logs remain as `*-initial-5315315.log`. STOP-SAVE and RESUME records are history; there is no active hold. Delta carries are declared in the ledger; the final full suite independently executes all current tests. Raw command logs preserve tool-generated trailing whitespace; source/documentation checks are clean.

Known limits retained from the approved packet: C6 p195 follows the disposed IR position limit; unmodeled drawing metadata and the previously observed overall page-count discrepancy remain outside C1. Raw flags required for the reported first-page box fix are preserved; **Parent Hancom visual verification and the separately dispatched Codex re-review remain subsequent stages**. No claim of restored total page count is made.

jev=used for changed source/test paths and the resumed header/ledger/manifest queries; all deterministic pin/exec candidates retained. Packet/dispositions returned jev=unused(candidate count 0), without excluding deterministic evidence.

INTENT RHWP-HWPX-SAVE-FIDELITY-L2-V353@r1 sha256:d385ca9b6e37df48347dfc71d28bfbc161c1db249304100a064347645b479366

- AC1: met in the approved fixture scope, with the disposed p195 exception; both added F1 tests pass on final source and fail at 8027a28.
- AC2: met; G3 and both existing two-save tests pass on final source.
- AC3: met; residual inventory and F1-F4 RED/GREEN are recorded, including exact raw flags preservation.
- AC4: met; final clean-source full suite/clippy/WASM pass, scope is zero outside the approved paths, ledger is 11/11, and six final-source review files exist.
