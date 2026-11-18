# C4: R4 F1 correction

Entry/source HEAD: c3be62605694758dd516d70802f958b32487ee91. Branch: v353-rhwp-table-fidelity-g0.

Removed only --locked from G3/G4/G5/G6/G9/G10. --offline resolves from this machine's cache in a clean isolated checkout. All EXPECTs and G8's CHECK, known failure and approved ABANDON are preserved. No product file, Cargo.lock or .gitignore change.

Canonical acceptance: `mah unlazy-rerun --repo "$PWD" --ledger gates/V353_RHWP_TABLE_FIDELITY_GATES.md --approve --isolated`, exit 0, elapsed 146s.

GATE_RERUN_SUMMARY pass=11 fail=0 nocheck=0 noevidence=0 abandoned=1 evidence_same=5 evidence_diff=6 worker_unmet=6 evidence_separator=0 evidence_truncation=0 evidence_substantive=6
GATE_RERUN_OK
ISOLATED_RERUN_EXIT=0

Final native gate status: 11 met + G8 abandoned; native rc1 is the explicit HANDOFF REQUIRED disposition. Comparison-row EVIDENCE fingerprints plus recorded shell/CWD are restored in the ledger. The canonical run's worker_unmet=6 reflects intermediate normalized evidence lacking shell metadata; final native status confirms that transport issue is closed. Definition/output evidence differences are preserved in the canonical log rather than described as failures.

Initial acceptance attempt: 11 PASS + G8 ABANDON, but source-local logging caused a transcript-only residue warning. Preserved as initial-rerun-logger-warning.log, excluded from acceptance. Canonical output was staged outside the source until the command returned and has no residue/cleanup warning.

Current-session findings-first review: no substantive C4 finding remains. Checked exact six CHECK deltas, all twelve EXPECTs, G8 ABANDON, native evidence state, source diff and ignored/untracked Cargo.lock policy. This is same-engine self-review, not independent assurance.

INTENT RHWP-TABLE-FIDELITY-P0-P2-V353@r1 sha256:c8d416ee6742068136815a06ef4ee606fd3a6b4e77a41875ef85ba68ae0eeaf7
AC1: met — original a34 baseline preserved; C3 G1 448/18 and canonical G2 zero loss reproduce; R4 independently confirms original 471/22.
AC2: met — canonical G4 passes; C3/R4 approved ROW-to-CELL interpretation and G0 tuple evidence retained.
AC3: met — canonical G5/G6 and G7 pass; C3/R4 source and negative-control evidence retained unchanged.
AC4: unmet — clean-checkout CHECK reproducibility F1 is closed and clippy/WASM/scope pass, but G8's approved pre-existing base failure remains ABANDON; no full-suite pass is claimed.
jev=used — ledger and canonical packet impact; deterministic pin/exec retained. C3/disposition queries returned no candidates. Same repo/task/scope intent binding verified current.

Remaining: Parent Hancom app confirmation and landing audit, and the existing G8 defect assigned to rhwp lane 2. The full cargo suite was not rerun in C4 because G8 remains explicitly abandoned. No push, PR or landing action.
