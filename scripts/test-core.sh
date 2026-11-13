#!/usr/bin/env sh
# Tests the dependency-free core even when crates.io is unavailable.
set -eu
cd "$(dirname "$0")/.."
build_dir=$(mktemp -d "${TMPDIR:-/tmp}/wifinx-tests.XXXXXX")
trap 'rm -rf "$build_dir"' EXIT HUP INT TERM
rustc --edition=2024 --deny warnings --crate-name wifinx_validator --crate-type lib src/lib.rs -o "$build_dir/libwifinx_validator.rlib"
for suite in protocols robustness services; do
    rustc --edition=2024 --deny warnings --test "tests/$suite.rs" --extern "wifinx_validator=$build_dir/libwifinx_validator.rlib" -o "$build_dir/$suite"
    if [ "${1:-}" = "--pure" ] && [ "$suite" = "services" ]; then
        "$build_dir/$suite" bounded_history_and_config
    else
        "$build_dir/$suite"
    fi
done
