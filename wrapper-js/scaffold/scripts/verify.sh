#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

find_python() {
  local candidate
  for candidate in python3.14 python3.13 python3.12 python3.11 python3.10 python3; do
    if command -v "$candidate" >/dev/null 2>&1; then
      if "$candidate" -c 'import sys; raise SystemExit(0 if sys.version_info >= (3, 10) else 1)' >/dev/null 2>&1; then
        printf '%s\n' "$candidate"
        return 0
      fi
    fi
  done
  printf 'verify.sh: no Python 3.10+ interpreter found on PATH\n' >&2
  return 1
}

PYTHON_BIN="$(find_python)"

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
run_step "publish:assert" node "$ROOT_DIR/scripts/assert-publish-readiness.mjs" publish
run_step "js:test" bash -lc "cd \"$ROOT_DIR/wrapper-js\" && npm test"
run_step "js:pack" bash -lc "cd \"$ROOT_DIR/wrapper-js\" && npm pack --dry-run"
run_step "py:test" bash -lc "cd \"$ROOT_DIR/wrapper-py\" && PYTHONPATH=src \"$PYTHON_BIN\" -m unittest discover -s tests -p 'test_*.py'"
