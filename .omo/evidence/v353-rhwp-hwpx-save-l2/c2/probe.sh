#!/usr/bin/env bash
set -euo pipefail

v353_repo=$(git rev-parse --show-toplevel)
v353_evidence="$v353_repo/.omo/evidence/v353-rhwp-hwpx-save-l2/c2"
v353_helper=.omo/evidence/v353-rhwp-hwpx-save-l2/c1/clean-cargo.sh
v353_base=7370ec9fdcf89eb4a372143bcb555ddd9bfe0b46

run() {
    local name=$1 cwd=$2 status=0
    shift 2
    {
        printf 'CWD=%s\nCOMMAND=' "$cwd"
        printf ' %q' "$@"
        printf '\n'
        (cd "$cwd" && "$@") || status=$?
        printf '\nEXIT=%s\n' "$status"
    } > "$v353_evidence/$name.log" 2>&1
    printf '%s exit=%s\n' "$name" "$status"
    return "$status"
}

assert_clean() {
    test "$(git -C "$1" rev-parse HEAD)" = "$v353_base"
    test -z "$(git -C "$1" status --porcelain -uall)"
    test -z "$(git -C "$1" ls-files Cargo.lock)"
    test ! -e "$1/Cargo.lock"
}

assert_cleanup() {
    test -z "$(ls -A "$v353_tmp/snapshots")"
}

case ${1:?Usage: bash probe.sh red|green|focused} in
red)
    v353_tmp=$(mktemp -d /tmp/v353-c2.XXXXXX)
    printf '%s\n' "$v353_tmp" > "$v353_evidence/probe-root.txt"
    mkdir "$v353_tmp/snapshots"
    run red-clone "$v353_tmp" git clone --shared --no-checkout "$v353_repo" "$v353_tmp/red"
    run red-checkout "$v353_tmp/red" git checkout --detach "$v353_base"
    assert_clean "$v353_tmp/red"
    run red-status "$v353_tmp/red" git status --porcelain -uall
    run cargo-version "$v353_tmp/red" cargo --version --verbose
    run rustc-version "$v353_tmp/red" rustc --version --verbose
    v353_status=0
    run red-no-lock "$v353_tmp/red" env TMPDIR="$v353_tmp/snapshots" bash "$v353_helper" --version || v353_status=$?
    test "$v353_status" -eq 1
    grep -q 'Cargo.lock: No such file or directory' "$v353_evidence/red-no-lock.log"
    test ! -e "$v353_tmp/red/target"
    assert_cleanup
    assert_clean "$v353_tmp/red"
    printf 'RED_OK: pristine exact-base helper failed before Cargo; snapshot removed\n'
    ;;
green)
    v353_tmp=$(cat "$v353_evidence/probe-root.txt")
    run green-clone "$v353_tmp" git clone --shared --no-checkout "$v353_repo" "$v353_tmp/green"
    run green-checkout "$v353_tmp/green" git checkout --detach "$v353_base"
    assert_clean "$v353_tmp/green"
    run green-initial-status "$v353_tmp/green" git status --porcelain -uall
    run green-no-lock "$v353_tmp/green" env TMPDIR="$v353_tmp/snapshots" bash "$v353_repo/$v353_helper" --version
    grep -q '^DEPENDENCY_INPUT=absent$' "$v353_evidence/green-no-lock.log"
    grep -q '^cargo 1\.' "$v353_evidence/green-no-lock.log"
    assert_cleanup
    assert_clean "$v353_tmp/green"
    v353_direct_status=0
    run direct-command-error "$v353_tmp/green" cargo --c2-invalid-option || v353_direct_status=$?
    v353_status=0
    run green-command-error "$v353_tmp/green" env TMPDIR="$v353_tmp/snapshots" bash "$v353_repo/$v353_helper" --c2-invalid-option || v353_status=$?
    test "$v353_direct_status" -ne 0
    test "$v353_status" -eq "$v353_direct_status"
    assert_cleanup
    assert_clean "$v353_tmp/green"
    cp "$v353_repo/Cargo.lock" "$v353_tmp/green/Cargo.lock"
    v353_lock_hash=$(shasum -a 256 "$v353_tmp/green/Cargo.lock" | cut -d ' ' -f1)
    run green-optional-lock "$v353_tmp/green" env TMPDIR="$v353_tmp/snapshots" bash "$v353_repo/$v353_helper" --version
    grep -q '^DEPENDENCY_INPUT=existing-Cargo.lock$' "$v353_evidence/green-optional-lock.log"
    grep -q "^DEPENDENCY_INPUT_SHA256=$v353_lock_hash$" "$v353_evidence/green-optional-lock.log"
    grep -q "^RESOLVED_CARGO_LOCK_SHA256=$v353_lock_hash$" "$v353_evidence/green-optional-lock.log"
    test "$(shasum -a 256 "$v353_tmp/green/Cargo.lock" | cut -d ' ' -f1)" = "$v353_lock_hash"
    rm "$v353_tmp/green/Cargo.lock"
    assert_cleanup
    assert_clean "$v353_tmp/green"
    chmod +x "$v353_evidence/fault-bin/git"
    v353_status=0
    run green-archive-error "$v353_tmp/green" env TMPDIR="$v353_tmp/snapshots" PATH="$v353_evidence/fault-bin:$PATH" bash "$v353_repo/$v353_helper" --version || v353_status=$?
    test "$v353_status" -eq 73
    test ! -e "$v353_tmp/green/target"
    assert_cleanup
    assert_clean "$v353_tmp/green"
    printf 'GREEN_OK: absent/optional lock, real Cargo error, archive pipefail and cleanup\n'
    ;;
focused)
    v353_tmp=$(cat "$v353_evidence/probe-root.txt")
    assert_clean "$v353_tmp/green"
    test ! -e "$v353_tmp/green/target"
    run focused-no-lock "$v353_tmp/green" env TMPDIR="$v353_tmp/snapshots" bash "$v353_repo/$v353_helper" test --offline --test hwpx_save_fidelity_l2 residual_c1_tab_halving_path_survives_two_saves -- --exact
    grep -q '^DEPENDENCY_INPUT=absent$' "$v353_evidence/focused-no-lock.log"
    grep -Eq '^RESOLVED_CARGO_LOCK_SHA256=[0-9a-f]{64}$' "$v353_evidence/focused-no-lock.log"
    grep -q 'test result: ok. 1 passed; 0 failed' "$v353_evidence/focused-no-lock.log"
    assert_cleanup
    assert_clean "$v353_tmp/green"
    printf 'FOCUSED_OK: one L2 test, absent lock, initially empty private target; snapshot removed\n'
    ;;
*)
    printf 'Unknown phase: %s\n' "$1" >&2
    exit 2
    ;;
esac
