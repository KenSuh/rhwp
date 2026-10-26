#!/usr/bin/env bash
set -euo pipefail

v354_source=$(git rev-parse --show-toplevel)
v354_evidence="$v354_source/.omo/evidence/v353-rhwp-hwpx-save-l2/final-v354"
v354_private=${1:?private root created for this dispatch}
v354_clone="$v354_private/checkout"
v354_candidate=17f85ce02d48a7da5d12a6e0997939949a60f613
v354_helper=.omo/evidence/v353-rhwp-hwpx-save-l2/c1/clean-cargo.sh
v354_checker=/Users/ken/.agents/skills/unlazy/scripts/gate-check.mjs
v354_relative=.omo/evidence/v353-rhwp-hwpx-save-l2/final-v354
export GIT_OPTIONAL_LOCKS=0 CARGO_NET_OFFLINE=true RUSTUP_AUTO_INSTALL=0
unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER

run() {
    local v354_name=$1 v354_cwd=$2 v354_started=$SECONDS v354_status=0
    shift 2
    printf 'START %s %s\n' "$v354_name" "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    {
        printf 'UTC_START=%s\nCWD=%s\nARGV=' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$v354_cwd"
        printf ' %q' "$@"
        printf '\n'
        (cd "$v354_cwd" && "$@") || v354_status=$?
        printf '\nUTC_END=%s\nELAPSED_SECONDS=%s\nEXIT_STATUS=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$((SECONDS-v354_started))" "$v354_status"
    } > "$v354_evidence/$v354_name.log" 2>&1
    printf 'END %s exit=%s elapsed=%ss\n' "$v354_name" "$v354_status" "$((SECONDS-v354_started))"
    return "$v354_status"
}

run checkout "$v354_clone" /usr/bin/git checkout --detach "$v354_candidate"
run pristine "$v354_clone" bash -euo pipefail -c '
    test "$(git rev-parse HEAD)" = 17f85ce02d48a7da5d12a6e0997939949a60f613
    test -z "$(git status --porcelain=v1 -uall)"
    test -z "$(git status --porcelain=v1 --ignored -uall)"
    test -z "$(git ls-files Cargo.lock)"
    test ! -e Cargo.lock
    test ! -e target
    git rev-parse HEAD "HEAD^{tree}"
    git status --porcelain=v1 --ignored -uall
    printf "PRISTINE_NO_TRACKED_OR_IGNORED_LOCK_NO_TARGET\n"
    shasum -a 256 Cargo.toml .omo/evidence/v353-rhwp-hwpx-save-l2/c1/clean-cargo.sh tests/hwpx_save_fidelity_l2.rs
'
mkdir -p "$v354_private/cargo-home" "$v354_private/snapshots"
run cache-copy "$v354_private" cp -cR /Users/ken/.cargo/registry "$v354_private/cargo-home/registry"
export CARGO_HOME="$v354_private/cargo-home" TMPDIR="$v354_private/snapshots"
run toolchains "$v354_clone" bash -euo pipefail -c '
    command -v cargo rustc rustup node
    cargo -Vv
    rustc -Vv
    rustup show active-toolchain
    rustup target list --installed
    rustup target list --installed | grep -Fx wasm32-unknown-unknown
    printf "CARGO_HOME=%s\nCARGO_NET_OFFLINE=%s\nTMPDIR=%s\n" "$CARGO_HOME" "$CARGO_NET_OFFLINE" "$TMPDIR"
    printf "RUSTFLAGS=unset\nCARGO_ENCODED_RUSTFLAGS=unset\nRUSTC_WRAPPER=unset\nRUSTC_WORKSPACE_WRAPPER=unset\n"
'
mkdir -p "$v354_clone/$v354_relative"
for v354_gate in 7 8 9; do
    run "g$v354_gate-input" "$v354_clone" bash -euo pipefail -c '
        git rev-parse HEAD "HEAD^{tree}"
        test ! -e Cargo.lock
        test -z "$(git diff --name-only HEAD)"
        shasum -a 256 .omo/evidence/v353-rhwp-hwpx-save-l2/c1/clean-cargo.sh
        if test -d target; then printf "TARGET_REUSE=private-existing\n"; else printf "TARGET_REUSE=absent\n"; fi
        printf "DEPENDENCY_INPUT=absent\n"
    '
    v354_status=0
    run "g$v354_gate-check" "$v354_clone" node "$v354_checker" --approve --cwd "$v354_clone" --timeout 3600 "$v354_evidence/G$v354_gate.md" || v354_status=$?
    for v354_log in "$v354_clone/$v354_relative/"*.log; do
        test -e "$v354_log" || continue
        cp "$v354_log" "$v354_evidence/"
    done
    orca orchestration check --terminal term_4551aa6b-3d4e-48fb-9c13-21f41611d6c7 --json
    if test "$v354_status" -ne 0; then
        printf 'FINAL_CHECK_FAILED=G%s\nNo heavy rerun was attempted.\n' "$v354_gate"
        exit "$v354_status"
    fi
done
run final-private-state "$v354_clone" bash -euo pipefail -c '
    git rev-parse HEAD "HEAD^{tree}"
    test ! -e Cargo.lock
    test -z "$(git diff --name-only HEAD)"
    git status --porcelain=v1 --ignored -uall
    test -z "$(find "$TMPDIR" -mindepth 1 -maxdepth 1 -print -quit)"
    printf "SNAPSHOTS_CLEANED\n"
'
printf 'FINAL_THREE_CHECKS_EXECUTED_ONCE\n'
