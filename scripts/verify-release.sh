#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

run_step() {
  local label="$1"
  shift
  printf '\n[%s]\n' "$label"
  "$@"
}

run_step "rust:fmt" cargo fmt --check --manifest-path "$ROOT_DIR/core-rs/Cargo.toml"
run_step "rust:clippy" cargo clippy --manifest-path "$ROOT_DIR/core-rs/Cargo.toml" -- -D warnings
run_step "rust:test" cargo test --manifest-path "$ROOT_DIR/core-rs/Cargo.toml"
run_step "tool:validate" cargo run --quiet --manifest-path "$ROOT_DIR/core-rs/Cargo.toml" -- validate --path "$ROOT_DIR" --json
run_step "tool:sync-check" cargo run --quiet --manifest-path "$ROOT_DIR/core-rs/Cargo.toml" -- sync --path "$ROOT_DIR" --check
run_step "release:assert" node "$ROOT_DIR/scripts/assert-release-state.mjs"
run_step "js:lockfile-assert" node "$ROOT_DIR/scripts/assert-js-lockfile-state.mjs"
run_step "publish:assert" node "$ROOT_DIR/scripts/assert-publish-readiness.mjs" publish
run_step "js:pack" bash -lc "cd \"$ROOT_DIR/wrapper-js\" && npm pack --dry-run"
