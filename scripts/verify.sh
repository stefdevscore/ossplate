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
JS_PACKAGE_NAME="$(node -p "require('$ROOT_DIR/wrapper-js/package.json').name")"
JS_VERSION="$(node -p "require('$ROOT_DIR/wrapper-js/package.json').version")"
if npm view "$JS_PACKAGE_NAME@$JS_VERSION" version >/dev/null 2>&1; then
  JS_LOCKFILE_MODE="resolved"
  JS_INSTALLABLE=true
else
  JS_LOCKFILE_MODE="placeholder"
  JS_INSTALLABLE=false
fi

run_step() {
  local label="$1"
  shift
  printf '\n[%s]\n' "$label"
  "$@"
}

run_step "rust:fmt" cargo fmt --check --manifest-path "$ROOT_DIR/core-rs/Cargo.toml"
run_step "rust:prepare-embedded-template" node "$ROOT_DIR/scripts/stage-distribution-assets.mjs" embedded-template
run_step "rust:clippy" cargo clippy --manifest-path "$ROOT_DIR/core-rs/Cargo.toml" -- -D warnings
run_step "rust:test" cargo test --manifest-path "$ROOT_DIR/core-rs/Cargo.toml"
run_step "tool:validate" cargo run --quiet --manifest-path "$ROOT_DIR/core-rs/Cargo.toml" -- validate --path "$ROOT_DIR" --json
run_step "tool:sync-check" cargo run --quiet --manifest-path "$ROOT_DIR/core-rs/Cargo.toml" -- sync --path "$ROOT_DIR" --check
run_step "release:plan-test" node --test "$ROOT_DIR/scripts/release-plan.test.mjs"
run_step "release:check-test" node --test "$ROOT_DIR/scripts/release-check.test.mjs"
run_step "release:state-test" node --test "$ROOT_DIR/scripts/release-state.test.mjs"
run_step "bootstrap:pattern1-test" node --test "$ROOT_DIR/scripts/bootstrap-pattern1.test.mjs"
run_step "publish:local-test" node --test "$ROOT_DIR/scripts/publish-local.test.mjs"
run_step "scaffold:assets-assert" node "$ROOT_DIR/scripts/release-check.mjs" scaffold-assets
run_step "release:assert" node "$ROOT_DIR/scripts/release-check.mjs" release-state
run_step "js:lockfile-assert" node "$ROOT_DIR/scripts/assert-js-lockfile-state.mjs" "$JS_LOCKFILE_MODE"
run_step "publish:assert" node "$ROOT_DIR/scripts/release-check.mjs" publish-readiness publish
if [ "$JS_INSTALLABLE" = true ]; then
  run_step "js:test" bash -lc "cd \"$ROOT_DIR/wrapper-js\" && npm test"
  run_step "js:pack" node "$ROOT_DIR/scripts/package-js.mjs" dry-run-json
else
  printf '\n[js:test]\n'
  printf 'skipped: current npm version %s is not published yet; placeholder lockfile state is expected\n' "$JS_VERSION"
fi
run_step "py:test" bash -lc "cd \"$ROOT_DIR/wrapper-py\" && PYTHONPATH=src \"$PYTHON_BIN\" -m unittest discover -s tests -p 'test_*.py'"
run_step "package:cleanliness" node "$ROOT_DIR/scripts/release-check.mjs" package-cleanliness
