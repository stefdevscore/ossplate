#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRIPT_NAME="$(basename "$0")"
MODE="${1:-all}"

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
  printf '%s: no Python 3.10+ interpreter found on PATH\n' "$SCRIPT_NAME" >&2
  return 1
}

PYTHON_BIN="$(find_python)"
WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/ossplate-live-e2e.XXXXXX")"
trap 'rm -rf "$WORK_DIR"' EXIT

run_step() {
  local label="$1"
  shift
  printf '\n[%s]\n' "$label"
  "$@"
}

assert_version_output() {
  local output="$1"
  if [[ "$output" != *'"tool":"ossplate"'* ]]; then
    printf '%s: unexpected version output: %s\n' "$SCRIPT_NAME" "$output" >&2
    return 1
  fi
}

assert_validate_ok() {
  local output="$1"
  if [[ "$output" != '{"ok":true,"issues":[]}' ]]; then
    printf '%s: unexpected validate output: %s\n' "$SCRIPT_NAME" "$output" >&2
    return 1
  fi
}

run_common_cli_flow() {
  local tool="$1"
  local target_root="$2"
  local create_dir="$target_root/created"
  local init_dir="$target_root/inited"

  mkdir -p "$target_root"

  local version_output
  version_output="$("$tool" version)"
  printf '%s\n' "$version_output"
  assert_version_output "$version_output"

  "$tool" create "$create_dir"
  local validate_output
  validate_output="$("$tool" validate --path "$create_dir" --json)"
  printf '%s\n' "$validate_output"
  assert_validate_ok "$validate_output"
  "$tool" sync --path "$create_dir" --check

  mkdir -p "$init_dir"
  "$tool" init --path "$init_dir"
  validate_output="$("$tool" validate --path "$init_dir" --json)"
  printf '%s\n' "$validate_output"
  assert_validate_ok "$validate_output"
  "$tool" sync --path "$init_dir" --check
}

run_cargo_flow() {
  local cargo_root="$WORK_DIR/cargo"
  local install_root="$cargo_root/root"
  local bin_path="$install_root/bin/ossplate"
  mkdir -p "$cargo_root"

  run_step "cargo:install" cargo install --root "$install_root" --force ossplate
  run_step "cargo:e2e" run_common_cli_flow "$bin_path" "$cargo_root"
}

run_npm_flow() {
  local npm_root="$WORK_DIR/npm"
  local tool_path
  mkdir -p "$npm_root"

  cat >"$npm_root/package.json" <<'JSON'
{
  "name": "ossplate-live-e2e",
  "private": true
}
JSON

  run_step "npm:install" bash -lc "cd \"$npm_root\" && npm install ossplate"
  if [[ "$OSTYPE" == msys* || "$OSTYPE" == cygwin* || "$(uname -s)" == MINGW* ]]; then
    tool_path="$npm_root/node_modules/.bin/ossplate.cmd"
  else
    tool_path="$npm_root/node_modules/.bin/ossplate"
  fi
  run_step "npm:e2e" run_common_cli_flow "$tool_path" "$npm_root"
}

run_python_flow() {
  local py_root="$WORK_DIR/python"
  local venv_dir="$py_root/venv"
  local pip_path="$venv_dir/bin/pip"
  local tool_path="$venv_dir/bin/ossplate"
  mkdir -p "$py_root"

  run_step "python:venv" "$PYTHON_BIN" -m venv "$venv_dir"
  run_step "python:install" "$pip_path" install --upgrade pip ossplate
  run_step "python:e2e" run_common_cli_flow "$tool_path" "$py_root"
}

case "$MODE" in
  cargo)
    run_cargo_flow
    ;;
  npm)
    run_npm_flow
    ;;
  python|py)
    run_python_flow
    ;;
  all)
    run_cargo_flow
    run_npm_flow
    run_python_flow
    ;;
  *)
    printf 'usage: %s [cargo|npm|python|all]\n' "$SCRIPT_NAME" >&2
    exit 1
    ;;
esac
