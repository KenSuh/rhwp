# rhwp L2 final clean-checkout verification

G7-G9 passed from the exact R3-approved candidate with no supplied Cargo.lock. The source ledger has 11 met gates, zero unmet gates and zero G7-G9 deferrals; eight historical gate blocks remain unchanged. This is a local verification/evidence checkpoint for Parent, not landing approval.

## Candidate, authority and checkout

- Tested candidate: `17f85ce02d48a7da5d12a6e0997939949a60f613`; tree `a1b872ba61d3e7e135754d188f253e840f31e030`.
- Source W: `/Users/ken/orca/workspaces/rhwp/v353-rhwp-table-fidelity`; branch `v353-rhwp-table-fidelity-g0`. Initial raw full status was empty, and the packet reservation was active with `claimed_by=v353-rhwp-hwpx-save-l2`.
- Parent release: 2026-10-26T13:09Z after task `task_4308f8063d2a` APPROVE. The whole R3 report was read and its SHA256 independently matched `1878c19fa4ea812526f35a7c3bd6382aaad2142cb7e18ceabc928f0eb0d02428`; R2, C2, original packet and approved dispositions were also read.
- A fresh 20-hour executor/repo lease bound the same canonical intent in the start command. The established reservation/task key is `v353-rhwp-hwpx-save-l2`; it is distinct from the intent ID. `intent-and-lease.log` reports `status=current`.
- Private clone: `/tmp/v353-final-v354.WRpWpU/checkout` (canonical `/private/tmp/...`). It was cloned locally with `mah git -- clone --no-hardlinks --no-checkout`, then privately checked out at exact candidate. `pristine.log` proves empty full/ignored status, no tracked or on-disk Cargo.lock, and no existing target before any check. The clone's source HEAD was never advanced.
- All three operations used the actual committed `.omo/evidence/v353-rhwp-hwpx-save-l2/c1/clean-cargo.sh` in this checkout: SHA256 `2b0e9279535d7656d764db8a7c13619eea7bab200a61b40b06ab32a5811ce1c6`. Each invocation created a fresh archive of candidate HEAD and removed its archive after execution.

The final local checkpoint is the commit containing this report and the final ledger. Its full SHA and the post-checkpoint source/helper equality result are returned in the worker handoff and harness context. No heavy check is repeated because this documentation creates a descendant commit.

## Actual three checks

`run-final.sh` ran the three inspected CHECK definitions serially through native unlazy single-gate ledgers `G7.md`, `G8.md`, `G9.md`. Each ledger's generated CHECK/EXPECT/EVIDENCE was transferred exactly to the matching source row; no evidence token was constructed by hand. Cargo argv is unchanged. `CARGO_NET_OFFLINE=true` preserves the original operations while disabling network access; no `--locked`, target substitution, dependency substitution or warning-suppression flag was added.

| Gate and actual Cargo argv | UTC start/end on 2026-10-26 | Whole CHECK elapsed | Actual Cargo / CHECK exit | Result and raw log |
|---|---|---:|---|---|
| G7 `test --no-fail-fast` | 13:18:05Z / 13:20:33Z | 148s | 0 / 0 | 1252 passed, 0 failed, 7 ignored; 17 groups; `g7-full.log` |
| G8 `clippy -- -D warnings` | 13:20:33Z / 13:20:51Z | 18s | 0 / 0 | Finished dev in 16.60s, no warnings; `g8-clippy.log` |
| G9 `build --lib --target wasm32-unknown-unknown --release` | 13:20:52Z / 13:21:21Z | 29s | 0 / 0 | Finished release in 28.24s; `g9-wasm.log` |

The wrapper logs are `g7-check.log`, `g8-check.log`, `g9-check.log`; each records exact argv, CWD, UTC, elapsed time and process exit. Cargo logs contain candidate identity, actual Cargo argv, target path, resolved-lock hash and actual Cargo status. `results.json` independently checks every group, both exit layers and byte equality between private raw logs and their committed copies. Failed groups: **none**. None of the three Cargo checks was retried.

| G7 group, in execution order | Passed | Failed | Ignored |
|---|---:|---:|---:|
| Library unit tests | 1001 | 0 | 1 |
| font_metric_gen unit tests | 0 | 0 | 0 |
| rhwp binary unit tests | 0 | 0 | 0 |
| cell_format_reflow | 17 | 0 | 0 |
| cell_reflow_geometry | 68 | 0 | 0 |
| cell_vertical_baseline | 2 | 0 | 0 |
| cell_vertical_geometry_matrix | 1 | 0 | 0 |
| form_caption_display | 1 | 0 | 0 |
| hwpx_header_resave_stability | 7 | 0 | 0 |
| hwpx_page_def_roundtrip | 13 | 0 | 0 |
| hwpx_phase1_corpus | 0 | 0 | 1 |
| hwpx_roundtrip_integration (G0) | 57 | 0 | 2 |
| hwpx_save_fidelity_l2 | 18 | 0 | 2 |
| hwpx_table_fidelity (lane 1) | 39 | 0 | 1 |
| hwpx_to_hwp_adapter | 25 | 0 | 0 |
| pagination_square_wrap | 3 | 0 | 0 |
| Doc tests | 0 | 0 | 0 |
| **Total** | **1252** | **0** | **7** |

G7's compile output preserves four existing library-test warnings: two non-snake-case test names and two unused Results. The requested G8 invocation emitted no warning. No source, manifest, test or lint policy was edited.

## Dependencies, toolchains and reuse

All three helper calls report absent dependency input and resolved lock SHA256 `e3bc5a141eb24b2ed63b11a6ddcb7979c855615e926cc8a575423f0e0c5ab8d1`; there is no divergence. It selects zip **9.1.1**, quick-xml **0.39.4**, wasm-bindgen **0.2.129**, web-sys/js-sys **0.3.106**. The old optional/local lock `a28b1372dcf6c2a945130ff6e1b6e5b43444f5b29f0d7b6b41847c003a351c93` selected zip 9.0.3 and was never copied into this checkout or the helper invocations.

After the checks, `cargo generate-lockfile --offline` in a separate archive of the same candidate produced an identical 36,465-byte lock. Its complete bytes are `resolved-Cargo.lock`; `dependencies.json` records all 158 packages (157 registry dependencies plus rhwp), versions, sources and checksums. `dependency-resolution.log` states this separate generation's provenance. Its matching digest identifies all three actually resolved locks; it did not provide their dependency input.

`cache-copy.log` records a private filesystem copy of the existing registry at `/tmp/v353-final-v354.WRpWpU/cargo-home/registry`. All actual checks used this private CARGO_HOME offline. The copy contains no symlinks. The live W target and another agent's registry/cache were not written by these Cargo operations. The helper forced the actual target to `/private/tmp/v353-final-v354.WRpWpU/checkout/target`: absent before G7, reused for G8 and G9. A new archive per invocation does not make later targets cold. Each archive independently resolved an absent lock. `g7-input.log`, `g8-input.log`, `g9-input.log` record this distinction; `final-private-state.log` confirms snapshot cleanup and unchanged tracked private source.

`toolchains.log`: cargo 1.95.0, commit `f2d3ce0bd7f24a49f8f72d9000448f8838c4e850`; rustc 1.95.0, commit `59807616e1fa2540724bfbac14d7976d7e4a3860`; host `aarch64-apple-darwin`, LLVM 22.1.2. Both native and `wasm32-unknown-unknown` targets were already installed. Rust flags/wrappers were unset; no toolchain/package installation occurred. Bash LSP was unavailable and installation was declined; `bash -n` passed for the evidence runner.

## Input equality, scope and carries

`reconciliation.json` and its exact read-only command `reconciliation-command.txt` establish:

- All 3,076 candidate Git entries other than the changed ledger match their on-disk Git blobs, including existing evidence and artifacts. The first attempt stopped on Git's quoted Korean filenames before verification completed; `reconciliation-initial-error.log` is retained. The successful command uses NUL-delimited tree entries. This was a cheap reconciliation correction, not a Cargo rerun.
- All product inputs under `src`, `tests`, Cargo.toml, build.rs, .cargo, template, assets and ttfs are unchanged from tested candidate and historical producer `a6d024f4f0bda4dcbe01bc247dceef003ea865d4`. Complete product listing SHA256 is `e37e82934c69b185df54c38abf1028a4872da80b2ca77c9d445b87f94072495a`; 433 selected source/helper/test/fixture/artifact/guardrail hashes are recorded individually. The L2 test remains SHA256 `31b58bbaf1a8d7a1426c192aa1113190d801c350124dc4ac89249aef517bd066`.
- G1-G6/G10/G11 blocks and all 11 EXPECT expressions remain byte-identical to candidate. Their historical evidence is carried with unchanged inputs and original dependency limits. G7 now executes the complete L2, G0, lane-1 and named G3 regression under the new clean-checkout resolution; that new execution is recorded in G7 rather than resealing the other eight rows.
- Baseline and latest negative-control input digests remain unchanged. `negative-control.json` still records original-base run-order failures=5, two-save failures=3 and residual failures=9. No negative suite or old CHECK was rerun.
- The final task delta is limited to the L2 ledger plus new `final-v354/**` evidence, as explicitly added to the packet by Parent's final brief. Original-base scope, with all approved dispositions, has zero out-of-scope paths. The V318 ledger, source/test/fixture/manifest bytes, existing evidence and Hancom artifacts are preserved.

The prior R3 approval carries because the corrected helper, product, tests, fixtures and all prior review/evidence inputs are unchanged. Only G7-G9 log destinations, checked states/evidence and final explanatory notes changed in the source ledger. R3 was a same-engine correction review and is not independent assurance, final cooperative-five evidence or a Parent landing decision.

## Hancom evidence and limits

All six existing artifacts still match `hancom/MANIFEST.txt` and the hashes in Parent's `/Users/ken/ken wiki/.omo/evidence/v338-vault-coordination/v354-rhwp-hancom/REVIEW.md` (SHA256 `5df875bee76724962c74738fc4b012ff3a502c6bd77b296cac27600e4170e469`). No artifact was regenerated and no native Hancom operation was performed in this task.

That prior receipt supports six successful opens and its C6 first-page observation. It records original C6=36 pages and C6 outputs=38/39/40, with FACF outputs=51 and the box on page 2. It does not establish whole-document visual equivalence, full page-count recovery or correct locations for every edit. The approved p195 IR-position exception, already unmodeled drawing metadata, FACF's previously lost flags/offsets and other listed layout limits remain as documented in the packet dispositions and retained ledger inventory. This verification adds no product fix.

## Original acceptance and handoff

INTENT RHWP-HWPX-SAVE-FIDELITY-L2-V353@r1 sha256:d385ca9b6e37df48347dfc71d28bfbc161c1db249304100a064347645b479366

AC1: met — `g7-full.log:1443` and `:1445` pass fixture order tests; the L2 group is 18/0/2, with body/cell/nested raw-order tests passing. Original-base failures remain in `../negative-control.json:7` with unchanged test digest. The approved p195 disposition remains explicit.

AC2: met — `g7-full.log:1004` passes `form_attribute_entities_survive_two_hwpx_saves`; `:1453` and `:1457` pass repeated-save section/column/page-number composition tests.

AC3: met — retained source-ledger G4 inventory measures fixed and remaining drawing/field/header defects; `g7-full.log:1441` through the residual/composition results ending at `:1459` pass their regressions. Original-base residual failures remain in `../negative-control.json:11`; R2's independently read raw-Hancom evidence and same-input review are carried. No new residual defect is claimed fixed.

AC4: met — actual G7 Cargo exit 0 and 17-group 1252/0/7, lane-1 39/0/1, G8/G9 actual exit 0 and Finished from the pristine checkout; zero out-of-scope paths; 11 valid source-ledger gates; six byte-matched Hancom review files. Raw logs, generated gate ledgers, `results.json`, `dependencies.json` and `reconciliation.json` supply the checks and identities.

jev=used — exact-candidate packet, helper, ledger, Cargo manifest/lock and Hancom-manifest impacts preceded protected reads/edits; all deterministic pin/exec candidates were retained in the impact logs. New-directory/disposition queries with zero candidates reported `jev=unused(0 candidates)` and are not counted as judgments. The helper's deterministic G7/G8/G9 bindings are all now directly verified.

The evidence runner owns only serial verification/log capture; the reconciliation reads inputs without product writes. No product/test/fixture/dependency edit, child, self-review/review-result receipt, push, merge, Actions or submodule change occurred. Executor assurance here is cooperative only. **Pending Parent work:** post-checkpoint input-equality audit, final cooperative five, critical-direct landing audit and any required exact-head CI/landing-close/landing decision. This task authorizes none of those landing operations. The executor lease is closed at handoff with the actual checkpoint recorded in harness context.
