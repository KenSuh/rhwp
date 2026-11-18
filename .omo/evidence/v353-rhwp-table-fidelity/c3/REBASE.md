# C3 — replay v353 onto the G0 chain

Result: **11/12 met + G8 ABANDON 1**. All twelve CHECKs were executed; the full cargo
suite is not green. Parent disposition 2 assigns the existing G0 section-run failure
to rhwp lane 2 and forbids fixing it in C3. This report is a handoff, not landing approval.

Task: v353-rhwp-table-fidelity, correction, local commits only.
Old branch: `v353-rhwp-table-fidelity` at `0052deae929c53830d2ff573c9c8ec817a4693b6` (retained).
Old base: `a34d88ff3bae317ad08d8f0b99504e4395278e6a`.
New base: `01a67b3f77760b5a9dae9fc155d83f8ca84ab529` (fetched tip verified).
New branch: `v353-rhwp-table-fidelity-g0`.
Tested source head: `dbf294757690e723c614c22e467022522ea3b44e`.
Later C3 commits record evidence only; the returned commit is identified in worker_done.

## Acceptance before verification

- Replay all eight lane commits in order; preserve all eleven G0 ancestors and the old branch.
- Resolve overlaps by behavior, preserving G0 fields, drawing, margins and border widths.
- Remeasure baseline/negative control on 01a67b3; change G1/G7/G11 base references and rerun all twelve gate CHECKs.
- Run lane and G0 Rust tests plus the G0 table-resize TypeScript regression.
- Regenerate the eight Hancom review files from the new base/head; app review remains Parent-owned.
- No new table-front spacing, drawing or field defect fixes, push, or self-review receipt.

## Replay and conflicts

`mah git switch -c v353-rhwp-table-fidelity-g0 0052dea` followed by `mah git rebase 01a67b3`
is the executor-allowed equivalent of replaying `a34d88ff..0052dea` onto the new base.
It replayed eight commits; the old branch was not rewritten. Only `45d4f16` conflicted:

1. `src/parser/hwpx/section.rs`, table attributes: retain G0's public `Table.no_adjust` field,
   v353's HWP5 bit-3 bridge, lock/textFlow parsing, and the disposition-2 pageBreak mapping.
2. `src/parser/hwpx/section.rs`, cell attributes: retain G0 `hasMargin` plus v353 protect/editable/dirty.
3. `src/serializer/hwpx/table.rs`, caption writer: keep G0's shared `shape::write_caption`,
   which already preserves all caption attributes and field-aware paragraphs; drop the duplicate lane writer.
4. `src/serializer/hwpx/table.rs`, cell paragraphs: keep G0 `write_paragraph` and
   `write_paragraph_text_runs` (fields, owner-run styles, supported controls), with the cell subList closing tag.

Non-conflicting overlap: the two noAdjust representations initially produced a shadowed serializer
binding. The G0 boolean remains the HWPX authority; HWP5 import now explicitly projects bit 3 into
that boolean, and retains the raw record for HWP5 preservation. The final mixed-format regression
fails before this bridge and passes afterward, for noAdjust both false and true.

The original conflict markers are in `conflicts-45d4f16.patch`; the complete 9,022-line
`git log -p a34d88ff..01a67b3` is archived in `g0-chain.patch`.

| Original lane commit | Replayed commit |
|---|---|
| 6e05aec | fce3e43 |
| 45d4f16 | 7be1fb0 |
| 1e87b0e | e984b3b |
| 00cb745 | e2aae65 |
| e6e5850 | 7ec640f |
| 7668ec2 | 64d18ca |
| 9ac673c | c812150 |
| 0052dea | e7c138e |

`replay-range-diff.txt` records that only the second replay changed. The source compatibility
commit is `dbf2947`, recorded in `c3-source.patch`; it adds the noAdjust import bridge/accessor
alignment, its regression, and the five authorized G0 test expectation rows.

## Semantic reconciliation and Parent decisions

- **noAdjust (0b12b89):** preserve the G0 public field and serializer. The lane raw bit-3 bridge
  remains for HWP5. `no-adjust-red-valid.log` is 0 passed/1 failed before the source bridge;
  `no-adjust-green.log` is 1 passed afterward. The preliminary `no-adjust-red.log` and
  `no-adjust-red-final.log` used incorrect expectations for a fixture with multiple sections;
  those attempts are preserved but excluded from the RED claim.
- **Border widths and field offsets (0a04914):** header, field, picture, and section serializers
  are byte-identical to the new base. G0 field/text-offset tests run in the full lib/integration suites.
- **Cell paragraphs, fields, native drawing (e81c3b2/730618c):** keep the G0 caption helper,
  field-aware paragraph/text-run functions, owner-run styles, and controls. Both table paragraph
  function spans are byte-identical to the base; their digests are in `verification.json`.
- **Paragraph margins/init order (4c279a1/c765a21):** header parser/serializer and section writer
  are unchanged from the new base, including the `hh:`/`hc:` margin behavior. The existing
  secPr/colPr idempotence failure is recorded below, without an additional correction.
- **Page-break expectation conflict:** G0's `c9_table_page_break_modes_and_legacy_aliases_survive`
  expected the opposite enum meanings from the approved lane disposition. Parent approved only
  five tuple changes and the G11 allowlist addition (`msg_322ceb2f6f8f`, canonical
  `outputs/v353-rhwp-table/RHWP_TABLE_FIDELITY_REBASE_C3_V353_DISPOSITION1.md`). TABLE maps to
  CellBreak and writes TABLE; CELL/ROW/ROW_BREAK/CELL_BREAK map to RowBreak and write CELL;
  NONE is unchanged. All assertions remain. The file is exactly +5/-5 from the base.

Twenty-seven G0-modified files are unchanged from the new base. The extra permitted
common_obj_attr_writer.rs change is still exactly +1/-0. No table-front spacing or remaining
drawing/field defects were started. No push or old-branch move was performed.

## Measurements and gate results

The ledger remains `gates/V353_RHWP_TABLE_FIDELITY_GATES.md`. All CHECKs and EXPECTs are retained;
only base literals, c3 output paths, G3 exit-preserving invocation, and the authorized G11 test path
were updated. No EXPECT was relaxed. C1/C2 notes remain as history and do not carry new-base gates.

| Gate | Measured result |
|---|---|
| G1 baseline | 01a67b3 T1=448, T2=18, total=466 attribute losses |
| G2 head preservation | T1/T2/T6/T7 ATTR_LOSS_TOTAL=0 |
| G3 serializer table | 29 passed |
| G4 page_break | 4 passed |
| G5 display_height | 18 passed |
| G6 width_formula | 14 passed |
| G7 negative control | Base page_break 2 passed/2 failed; display_height 8 passed/10 failed |
| G8 cargo test | Failed: lib 1000 passed/1 failed/1 ignored; base failure reproduced |
| G9 clippy -D warnings | exit 0 |
| G10 wasm32 release | exit 0 |
| G11 scope | OUT_OF_SCOPE=0 |
| G12 Hancom review artifacts | 8 regenerated files, ZIP/XML and hashes checked |

The base measurements use an isolated `git archive`. All 2817 tracked blobs were checked
against 01a67b3 before measuring. The archive has no .git: baseline.json retains the observed
HEAD null and separately binds the verified base. The identical Cargo.toml and copied, identical
Cargo.lock allow offline/locked execution. `base-source-binding.json`, `measure_base.py`, and
`measure-base.log` record the method. The old a34d88ff baseline (471/22) remains in lane history.

Only two G6 tests require the lane-only set_column_widths API and prevented compiling the test
file on the base. In the archive alone they receive `#[cfg(any())]`; G4/G5 and production code
are unchanged. `negative-control.json` records both test-file digests and names. Additional
base width negative control is 3 passed/9 failed; it is not represented as all 14 width cases.

## Existing-base finding and completed test inventory

`serializer::hwpx::tests::form_attribute_entities_survive_two_hwpx_saves` fails at
`src/serializer/hwpx/mod.rs:430`. First save places colPr in the secPr run; second save creates
a separate run. Attribute/entity assertions pass and the final whole-section equality fails.
The unchanged, blob-verified 01a67b3 archive fails the same test with the identical minimal XML
diff (`g0-base-form-failure.log` and the two `*.xml-diff.txt` files). Both serializer mod.rs and
section.rs equal the base. This is evidence of an existing G0 failure, not evidence that the
whole G0 baseline suite is green.

Parent `msg_fc62bcbcfa2c` / canonical
`outputs/v353-rhwp-table/RHWP_TABLE_FIDELITY_REBASE_C3_V353_DISPOSITION2.md` explicitly assigns
this issue to rhwp lane 2. The exact required ABANDON line is in the ledger. The ledger status
is HANDOFF REQUIRED, and the gate checker exits 1; lint passes.

Because cargo test stopped after the failing lib target, every remaining cargo target was
explicitly completed with the same source inputs. Counts below exclude the separate filtered
gate reruns and the artifact writer, avoiding double-counting.

| Target/run | Passed | Failed | Ignored |
|---|---:|---:|---:|
| lib (G8) | 1000 | 1 | 1 |
| lane hwpx_table_fidelity | 39 | 0 | 1 |
| G0 hwpx_roundtrip_integration | 57 | 0 | 2 |
| G0 hwpx_header_resave_stability | 7 | 0 | 0 |
| remaining 2 bins + 9 integration targets | 130 | 0 | 1 |
| doctests | 0 | 0 | 0 |
| **Rust total, 16 result bundles** | **1233** | **1** | **5** |
| G0 Studio table-resize (Bun, separate) | 24 | 0 | 0 |

Commands and output: `g8.log`, `lane-g0-integration.log`, `remaining-targets-command.json`,
`remaining-targets.log`, `doctests.log`, and `g0-studio.log`. The three selected integration
targets ran with `cargo test --offline --locked --test hwpx_roundtrip_integration
--test hwpx_header_resave_stability --test hwpx_table_fidelity`; doctests used
`cargo test --offline --locked --doc`; Studio used
`bun test rhwp-studio/tests/table-resize-renderer.test.ts`.

Two private G0 fixtures are ignored; other optional private-input tests can return early when
inputs are absent. These standard-suite counts do not claim private-document coverage.
The lane's ignored artifact writer was run separately for six head and two base HWPX files.
Their actual dimensions and hashes are in `hancom-manifest.json` and `../hancom/README.md`.
Hancom application review remains Parent-owned.

## Current-session audit and intent

The current Codex session directly reviewed the replay range diff, all four conflict resolutions,
the noAdjust bridge/accessor, and the authorized five tuple rows. G0-owned writer paths and
27 unchanged G0 blobs were compared with the base. No further source finding was identified
in this audit; the G8 failure above remains unresolved. This is cooperative executor evidence,
not independent review; no self-review receipt was created. Parent review/landing audit remains.

`verification.json` remeasures branch/ancestry/replay identity, source/tree and fixture hashes,
gate output fingerprints, totals, base binding, and the identical failure diff. Source/test
inputs match the tested source head; evidence-only changes do not justify repeating heavy gates.
Unfiltered staged `git diff --check` reports whitespace in verbatim patch/log transcripts only
(`whitespace-raw.log`); excluding those raw transcripts passes. Their bytes are preserved so
recorded output fingerprints remain valid.

INTENT RHWP-TABLE-FIDELITY-P0-P2-V353@r1 sha256:c8d416ee6742068136815a06ef4ee606fd3a6b4e77a41875ef85ba68ae0eeaf7

AC1: met — original `../baseline.json` retained; new-base `baseline.json` 448/18 and `g2.log:14` zero losses.
AC2: met — `g4.log:10` 4 passed, renderer/HWP5 mapping evidence retained, authorized ROW→CELL normalization.
AC3: met — `g5.log:24` / `g6.log:20` 18/14 passed; `negative-control.json` base page_break/display_height fail 2/10 cases.
AC4: unmet — `g8.log:1063` existing-base failure; `g9.log:2`, `g10.log:2`, scope and remaining cargo targets pass.

jev=used (deterministic pin/exec dependencies retained).
