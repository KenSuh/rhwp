#!/usr/bin/env bash
set -euo pipefail

v353_repo=$(git rev-parse --show-toplevel)
v353_head=$(git rev-parse HEAD)
v353_snapshot=$(mktemp -d "${TMPDIR:-/tmp}/v353-c1-clean.XXXXXX")
trap 'rm -rf "$v353_snapshot"' EXIT
git archive "$v353_head" | tar -x -C "$v353_snapshot"
cp "$v353_repo/Cargo.lock" "$v353_snapshot/Cargo.lock"
printf 'CLEAN_SOURCE_HEAD=%s\nCLEAN_SOURCE_METHOD=git-archive+identical-Cargo.lock\n' "$v353_head"
printf 'COMMAND=cargo'
printf ' %q' "$@"
printf '\n'
export CARGO_TARGET_DIR="$v353_repo/target"
cd "$v353_snapshot"
cargo "$@"
