#!/usr/bin/env bash
set -euo pipefail

v353_repo=$(git rev-parse --show-toplevel)
v353_head=$(git rev-parse HEAD)
v353_snapshot=$(mktemp -d "${TMPDIR:-/tmp}/v353-c1-clean.XXXXXX")
trap 'rm -rf "$v353_snapshot"' EXIT
git archive "$v353_head" | tar -x -C "$v353_snapshot"
printf 'CLEAN_SOURCE_HEAD=%s\nCLEAN_SOURCE_METHOD=git-archive\n' "$v353_head"
if [[ -f "$v353_repo/Cargo.lock" ]]; then
    cp "$v353_repo/Cargo.lock" "$v353_snapshot/Cargo.lock"
    printf 'DEPENDENCY_INPUT=existing-Cargo.lock\nDEPENDENCY_INPUT_SHA256='
    shasum -a 256 "$v353_snapshot/Cargo.lock" | cut -d ' ' -f1
else
    printf 'DEPENDENCY_INPUT=absent\nDEPENDENCY_INPUT_SHA256=absent\n'
fi
printf 'COMMAND=cargo'
printf ' %q' "$@"
printf '\n'
export CARGO_TARGET_DIR="$v353_repo/target"
printf 'CARGO_TARGET_DIR=%s\n' "$CARGO_TARGET_DIR"
cd "$v353_snapshot"
v353_cargo_status=0
cargo "$@" || v353_cargo_status=$?
printf 'RESOLVED_CARGO_LOCK_SHA256='
if [[ -f Cargo.lock ]]; then
    shasum -a 256 Cargo.lock | cut -d ' ' -f1
else
    printf 'absent\n'
fi
printf 'CARGO_EXIT_STATUS=%s\n' "$v353_cargo_status"
exit "$v353_cargo_status"
