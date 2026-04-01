#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRIPT_NAME="$(basename "$0")"
MODE="${1:-all}"
CAPTURE_DIR="${OSSPLATE_LIVE_E2E_CAPTURE_DIR:-$ROOT_DIR/.live-e2e}"
TIMESTAMP="$(date +"%Y%m%d-%H%M%S")"
CAPTURE_FILE="$CAPTURE_DIR/live-e2e-$MODE-$TIMESTAMP.log"
NPM_PACKAGE_SPEC="${OSSPLATE_LIVE_E2E_NPM_PACKAGE_SPEC:-ossplate}"
NPM_RUNTIME_SPEC="${OSSPLATE_LIVE_E2E_NPM_RUNTIME_SPEC:-}"
SCOPED_NPM_PACKAGE="${OSSPLATE_LIVE_E2E_SCOPED_NPM_PACKAGE:-@acme/blade-live}"
CARGO_INSTALL_SPEC="${OSSPLATE_LIVE_E2E_CARGO_INSTALL_SPEC:-ossplate}"
PYTHON_PACKAGE_SPEC="${OSSPLATE_LIVE_E2E_PYTHON_PACKAGE_SPEC:-ossplate}"

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

mkdir -p "$CAPTURE_DIR"
redact_capture_stream() {
  sed -E \
    -e 's/(NPM_TOKEN=)[^[:space:]]+/\1[REDACTED]/g' \
    -e 's/(CARGO_REGISTRY_TOKEN=)[^[:space:]]+/\1[REDACTED]/g' \
    -e 's/(TWINE_PASSWORD=)[^[:space:]]+/\1[REDACTED]/g' \
    -e 's/(TWINE_USERNAME=)[^[:space:]]+/\1[REDACTED]/g' \
    -e 's/(Authorization:[[:space:]]*Bearer[[:space:]]+)[^[:space:]]+/\1[REDACTED]/Ig' \
    -e 's/([?&](token|authToken|password)=)[^&[:space:]]+/\1[REDACTED]/Ig'
}
exec > >(redact_capture_stream | tee "$CAPTURE_FILE") 2>&1

printf '[capture]\n%s\n' "$CAPTURE_FILE"
printf '[workdir]\n%s\n' "$WORK_DIR"

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

assert_validate_with_mode() {
  local output="$1"
  local mode="$2"
  local status
  status="$("$PYTHON_BIN" -c '
import json
import sys

try:
    payload = json.loads(sys.argv[1])
except Exception:
    print("invalid")
    raise SystemExit(0)

mode = sys.argv[2]
ok = payload.get("ok") is True
issues = payload.get("issues")
warnings = payload.get("warnings")

placeholder_warnings = [
    "project.description still uses the generated placeholder; replace it before release",
    "project.repository still uses the generated placeholder; set the real repository URL before release",
    "author.name still uses the generated placeholder; set the real maintainer name before release",
    "author.email still uses the generated placeholder; set the real maintainer email before release",
]

if not ok or issues != []:
    print("invalid")
elif mode == "placeholder":
    print("ok" if warnings == placeholder_warnings else "invalid")
elif mode == "clean":
    print("ok" if warnings == [] else "invalid")
else:
    print("invalid")
' "$output" "$mode")"
  if [[ "$status" != "ok" ]]; then
    printf '%s: unexpected validate output: %s\n' "$SCRIPT_NAME" "$output" >&2
    return 1
  fi
}

assert_sync_ok() {
  local output="$1"
  local status
  status="$("$PYTHON_BIN" -c '
import json
import sys

try:
    payload = json.loads(sys.argv[1])
except Exception:
    print("invalid")
    raise SystemExit(0)

ok = payload.get("ok") is True
changes = payload.get("changedFiles")
print("ok" if ok and (changes == [] or changes is None) else "invalid")
' "$output")"
  if [[ "$status" != "ok" ]]; then
    printf '%s: unexpected sync output: %s\n' "$SCRIPT_NAME" "$output" >&2
    return 1
  fi
}

assert_inspect_current() {
  local output="$1"
  local project_name="$2"
  local rust_crate="$3"
  local npm_package="$4"
  local python_package="$5"
  local command_name="$6"
  local status
  status="$("$PYTHON_BIN" -c '
import json
import sys

try:
    payload = json.loads(sys.argv[1])
except Exception:
    print("invalid")
    raise SystemExit(0)

project_name, rust_crate, npm_package, python_package, command_name = sys.argv[2:7]
config = payload.get("config", {})
packages = config.get("packages", {})
project = config.get("project", {})
derived = payload.get("derived", {})
paths = derived.get("paths", {})
runtime_packages = derived.get("runtimePackages", [])

expected_runtime_names = {
    f"{npm_package}-darwin-arm64",
    f"{npm_package}-darwin-x64",
    f"{npm_package}-linux-x64",
    f"{npm_package}-windows-x64",
}
actual_runtime_names = {entry.get("packageName") for entry in runtime_packages}

checks = [
    payload.get("compatibility") == "current",
    payload.get("scaffoldVersion") == payload.get("latestScaffoldVersion"),
    payload.get("upgradePath") == [],
    project.get("name") == project_name,
    packages.get("rust_crate") == rust_crate,
    packages.get("npm_package") == npm_package,
    packages.get("python_package") == python_package,
    packages.get("command") == command_name,
    paths.get("pythonModule") == python_package,
    paths.get("pythonEntrypoint") == f"{python_package}.cli:main",
    paths.get("pythonPackageDir") == f"wrapper-py/src/{python_package}",
    paths.get("pythonCliModulePath") == f"wrapper-py/src/{python_package}/cli.py",
    paths.get("jsWrapperLauncher") == f"wrapper-js/bin/{command_name}.js",
    actual_runtime_names == expected_runtime_names,
]

print("ok" if all(checks) else "invalid")
' "$output" "$project_name" "$rust_crate" "$npm_package" "$python_package" "$command_name")"
  if [[ "$status" != "ok" ]]; then
    printf '%s: unexpected inspect output: %s\n' "$SCRIPT_NAME" "$output" >&2
    return 1
  fi
}

assert_inspect_upgrade_supported() {
  local output="$1"
  local from_version="$2"
  local expected_path_json="$3"
  local status
  status="$("$PYTHON_BIN" -c '
import json
import sys

try:
    payload = json.loads(sys.argv[1])
except Exception:
    print("invalid")
    raise SystemExit(0)

from_version = int(sys.argv[2])
expected_path = json.loads(sys.argv[3])

checks = [
    payload.get("scaffoldVersion") == from_version,
    payload.get("latestScaffoldVersion") == 3,
    payload.get("compatibility") == "upgrade_supported",
    payload.get("recommendedAction") == "upgrade",
    payload.get("upgradePath") == expected_path,
    payload.get("blockingReason") is None,
]
print("ok" if all(checks) else "invalid")
' "$output" "$from_version" "$expected_path_json")"
  if [[ "$status" != "ok" ]]; then
    printf '%s: unexpected upgrade inspect output: %s\n' "$SCRIPT_NAME" "$output" >&2
    return 1
  fi
}

assert_upgrade_plan() {
  local output="$1"
  local from_version="$2"
  local to_version="$3"
  local expected_path_json="$4"
  local step_count="$5"
  local required_changed_path="$6"
  local status
  status="$("$PYTHON_BIN" -c '
import json
import sys

try:
    payload = json.loads(sys.argv[1])
except Exception:
    print("invalid")
    raise SystemExit(0)

from_version = int(sys.argv[2])
to_version = int(sys.argv[3])
expected_path = json.loads(sys.argv[4])
step_count = int(sys.argv[5])
required_changed_path = sys.argv[6]

step_plans = payload.get("stepPlans") or []
changed_files = payload.get("changedFiles") or []

checks = [
    payload.get("ok") is True,
    payload.get("apply") is False,
    payload.get("fromVersion") == from_version,
    payload.get("toVersion") == to_version,
    payload.get("compatibility") == "upgrade_supported",
    payload.get("canApply") is True,
    payload.get("upgradePath") == expected_path,
    len(step_plans) == step_count,
    required_changed_path in changed_files,
]
print("ok" if all(checks) else "invalid")
' "$output" "$from_version" "$to_version" "$expected_path_json" "$step_count" "$required_changed_path")"
  if [[ "$status" != "ok" ]]; then
    printf '%s: unexpected upgrade plan output: %s\n' "$SCRIPT_NAME" "$output" >&2
    return 1
  fi
}

assert_upgrade_apply() {
  local output="$1"
  local from_version="$2"
  local to_version="$3"
  local expected_path_json="$4"
  local required_changed_path="$5"
  local status
  status="$("$PYTHON_BIN" -c '
import json
import sys

try:
    payload = json.loads(sys.argv[1])
except Exception:
    print("invalid")
    raise SystemExit(0)

from_version = int(sys.argv[2])
to_version = int(sys.argv[3])
expected_path = json.loads(sys.argv[4])
required_changed_path = sys.argv[5]
changed_files = payload.get("changedFiles") or []

checks = [
    payload.get("ok") is True,
    payload.get("apply") is True,
    payload.get("fromVersion") == from_version,
    payload.get("toVersion") == to_version,
    payload.get("upgradePath") == expected_path,
    required_changed_path in changed_files,
]
print("ok" if all(checks) else "invalid")
' "$output" "$from_version" "$to_version" "$expected_path_json" "$required_changed_path")"
  if [[ "$status" != "ok" ]]; then
    printf '%s: unexpected upgrade apply output: %s\n' "$SCRIPT_NAME" "$output" >&2
    return 1
  fi
}

assert_inspect_unsupported() {
  local output="$1"
  local reason_substring="$2"
  local status
  status="$("$PYTHON_BIN" -c '
import json
import sys

try:
    payload = json.loads(sys.argv[1])
except Exception:
    print("invalid")
    raise SystemExit(0)

reason_substring = sys.argv[2]
blocking_reason = payload.get("blockingReason") or ""

checks = [
    payload.get("compatibility") == "unsupported",
    payload.get("recommendedAction") == "stop",
    payload.get("upgradePath") == [],
    reason_substring in blocking_reason,
]
print("ok" if all(checks) else "invalid")
' "$output" "$reason_substring")"
  if [[ "$status" != "ok" ]]; then
    printf '%s: unexpected unsupported inspect output: %s\n' "$SCRIPT_NAME" "$output" >&2
    return 1
  fi
}

assert_command_failure_contains() {
  local output="$1"
  local expected="$2"
  if [[ "$output" != *"$expected"* ]]; then
    printf '%s: expected failure output to contain %s, got: %s\n' "$SCRIPT_NAME" "$expected" "$output" >&2
    return 1
  fi
}

downgrade_descendant_to_version() {
  local root="$1"
  local version="$2"

  case "$version" in
    2)
      "$PYTHON_BIN" - <<'PY' "$root"
import json
import pathlib

root = pathlib.Path(__import__("sys").argv[1])

contents = root.joinpath("ossplate.toml").read_text(encoding="utf-8")
root.joinpath("ossplate.toml").write_text(
    contents.replace("scaffold_version = 3", "scaffold_version = 2"),
    encoding="utf-8",
)

target = root / "core-rs/src/upgrade_catalog.rs"
if target.exists():
    target.unlink()

for manifest_rel in ["scaffold-payload.json", "source-checkout.json", "core-rs/source-checkout.json"]:
    path = root / manifest_rel
    data = json.loads(path.read_text())
    data["requiredPaths"] = [p for p in data["requiredPaths"] if p != "core-rs/src/upgrade_catalog.rs"]
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
PY
      ;;
    1)
      downgrade_descendant_to_version "$root" 2
      "$PYTHON_BIN" - <<'PY' "$root"
import json
import pathlib

root = pathlib.Path(__import__("sys").argv[1])

contents = root.joinpath("ossplate.toml").read_text(encoding="utf-8")
root.joinpath("ossplate.toml").write_text(
    contents.replace("scaffold_version = 2", "scaffold_version = 1"),
    encoding="utf-8",
)

removed = [
    "core-rs/build.rs",
    "core-rs/src/embedded_template.rs",
    "core-rs/src/upgrade.rs",
    "core-rs/src/verify.rs",
    "scripts/stage-embedded-template.mjs",
    "scripts/package-js.mjs",
]
for rel in removed:
    path = root / rel
    if path.exists():
        path.unlink()

for manifest_rel in ["scaffold-payload.json", "source-checkout.json", "core-rs/source-checkout.json"]:
    path = root / manifest_rel
    data = json.loads(path.read_text())
    data["requiredPaths"] = [p for p in data["requiredPaths"] if p not in removed]
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
PY
      ;;
    *)
      printf '%s: unsupported downgrade version %s\n' "$SCRIPT_NAME" "$version" >&2
      return 1
      ;;
  esac
}

drift_historical_descendant_contract() {
  local root="$1"
  "$PYTHON_BIN" - <<'PY' "$root"
import json
import pathlib

root = pathlib.Path(__import__("sys").argv[1])
path = root / "core-rs/source-checkout.json"
data = json.loads(path.read_text())
data["requiredPaths"] = [p for p in data["requiredPaths"] if p != "core-rs/src/output.rs"]
path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
PY
}

run_generated_repo_verify() {
  local repo_root="$1"
  run_step "generated:verify" bash -lc "cd \"$repo_root\" && ./scripts/verify.sh"
}

run_upgrade_flow() {
  local tool="$1"
  local target_root="$2"
  local v2_dir="$target_root/upgrade-v2"
  local v1_dir="$target_root/upgrade-v1"
  local drifted_dir="$target_root/upgrade-drifted"
  local inspect_output
  local plan_output
  local apply_output
  local validate_output
  local sync_output
  local failure_output

  "$tool" create "$v2_dir" \
    --name "upgrade-v2" \
    --description "Live upgrade fixture." \
    --repository "https://example.com/upgrade-v2" \
    --license "Apache-2.0" \
    --author-name "Upgrade Fixture" \
    --author-email "upgrade@example.com" \
    --rust-crate "upgrade_v2" \
    --npm-package "upgrade-v2" \
    --python-package "upgrade_v2" \
    --command "upgrade-v2"
  downgrade_descendant_to_version "$v2_dir" 2
  inspect_output="$("$tool" inspect --path "$v2_dir" --json)"
  printf '%s\n' "$inspect_output"
  assert_inspect_upgrade_supported "$inspect_output" 2 '["2->3"]'
  plan_output="$("$tool" upgrade --path "$v2_dir" --plan --json)"
  printf '%s\n' "$plan_output"
  assert_upgrade_plan "$plan_output" 2 3 '["2->3"]' 1 "core-rs/src/upgrade_catalog.rs"
  apply_output="$("$tool" upgrade --path "$v2_dir" --json)"
  printf '%s\n' "$apply_output"
  assert_upgrade_apply "$apply_output" 2 3 '["2->3"]' "core-rs/src/upgrade_catalog.rs"
  validate_output="$("$tool" validate --path "$v2_dir" --json)"
  printf '%s\n' "$validate_output"
  assert_validate_with_mode "$validate_output" "clean"
  inspect_output="$("$tool" inspect --path "$v2_dir" --json)"
  printf '%s\n' "$inspect_output"
  assert_inspect_current "$inspect_output" "upgrade-v2" "upgrade_v2" "upgrade-v2" "upgrade_v2" "upgrade-v2"
  sync_output="$("$tool" sync --path "$v2_dir" --check --json)"
  printf '%s\n' "$sync_output"
  assert_sync_ok "$sync_output"

  "$tool" create "$v1_dir" \
    --name "upgrade-v1" \
    --description "Live chained upgrade fixture." \
    --repository "https://example.com/upgrade-v1" \
    --license "Apache-2.0" \
    --author-name "Upgrade Fixture" \
    --author-email "upgrade@example.com" \
    --rust-crate "upgrade_v1" \
    --npm-package "upgrade-v1" \
    --python-package "upgrade_v1" \
    --command "upgrade-v1"
  downgrade_descendant_to_version "$v1_dir" 1
  inspect_output="$("$tool" inspect --path "$v1_dir" --json)"
  printf '%s\n' "$inspect_output"
  assert_inspect_upgrade_supported "$inspect_output" 1 '["1->2","2->3"]'
  plan_output="$("$tool" upgrade --path "$v1_dir" --plan --json)"
  printf '%s\n' "$plan_output"
  assert_upgrade_plan "$plan_output" 1 3 '["1->2","2->3"]' 2 "core-rs/src/upgrade.rs"
  apply_output="$("$tool" upgrade --path "$v1_dir" --json)"
  printf '%s\n' "$apply_output"
  assert_upgrade_apply "$apply_output" 1 3 '["1->2","2->3"]' "core-rs/src/upgrade_catalog.rs"
  validate_output="$("$tool" validate --path "$v1_dir" --json)"
  printf '%s\n' "$validate_output"
  assert_validate_with_mode "$validate_output" "clean"
  inspect_output="$("$tool" inspect --path "$v1_dir" --json)"
  printf '%s\n' "$inspect_output"
  assert_inspect_current "$inspect_output" "upgrade-v1" "upgrade_v1" "upgrade-v1" "upgrade_v1" "upgrade-v1"
  sync_output="$("$tool" sync --path "$v1_dir" --check --json)"
  printf '%s\n' "$sync_output"
  assert_sync_ok "$sync_output"

  "$tool" create "$drifted_dir" \
    --name "upgrade-drifted" \
    --description "Live drift refusal fixture." \
    --repository "https://example.com/upgrade-drifted" \
    --license "Apache-2.0" \
    --author-name "Upgrade Fixture" \
    --author-email "upgrade@example.com" \
    --rust-crate "upgrade_drifted" \
    --npm-package "upgrade-drifted" \
    --python-package "upgrade_drifted" \
    --command "upgrade-drifted"
  downgrade_descendant_to_version "$drifted_dir" 2
  drift_historical_descendant_contract "$drifted_dir"
  inspect_output="$("$tool" inspect --path "$drifted_dir" --json)"
  printf '%s\n' "$inspect_output"
  assert_inspect_unsupported "$inspect_output" "does not match"
  if failure_output="$("$tool" upgrade --path "$drifted_dir" --json 2>&1)"; then
    printf '%s: expected upgrade refusal for drifted historical descendant\n' "$SCRIPT_NAME" >&2
    return 1
  fi
  printf '%s\n' "$failure_output"
  assert_command_failure_contains "$failure_output" "does not match"
}

run_scoped_identity_flow() {
  local tool="$1"
  local target_root="$2"
  local scoped_dir="$target_root/scoped"
  local validate_output
  local inspect_output
  local sync_output

  "$tool" create "$scoped_dir" \
    --name "Scoped Blade" \
    --description "Live E2E scoped identity fixture." \
    --repository "https://example.com/scoped-blade" \
    --license "Apache-2.0" \
    --author-name "Live E2E" \
    --author-email "live-e2e@example.com" \
    --rust-crate "blade_scope" \
    --npm-package "$SCOPED_NPM_PACKAGE" \
    --python-package "blade_scope" \
    --command "blade-scope"
  validate_output="$("$tool" validate --path "$scoped_dir" --json)"
  printf '%s\n' "$validate_output"
  assert_validate_with_mode "$validate_output" "clean"
  inspect_output="$("$tool" inspect --path "$scoped_dir" --json)"
  printf '%s\n' "$inspect_output"
  assert_inspect_current "$inspect_output" "Scoped Blade" "blade_scope" "$SCOPED_NPM_PACKAGE" "blade_scope" "blade-scope"
  sync_output="$("$tool" sync --path "$scoped_dir" --check --json)"
  printf '%s\n' "$sync_output"
  assert_sync_ok "$sync_output"
}

run_common_cli_flow() {
  local tool="$1"
  local target_root="$2"
  local create_dir="$target_root/created"
  local init_dir="$target_root/inited"
  local custom_dir="$target_root/customized"

  mkdir -p "$target_root"

  local version_output
  version_output="$("$tool" version)"
  printf '%s\n' "$version_output"
  assert_version_output "$version_output"

  "$tool" create "$create_dir"
  local validate_output
  local inspect_output
  local sync_output
  validate_output="$("$tool" validate --path "$create_dir" --json)"
  printf '%s\n' "$validate_output"
  assert_validate_with_mode "$validate_output" "placeholder"
  inspect_output="$("$tool" inspect --path "$create_dir" --json)"
  printf '%s\n' "$inspect_output"
  assert_inspect_current "$inspect_output" "Ossplate" "ossplate" "ossplate" "ossplate" "ossplate"
  sync_output="$("$tool" sync --path "$create_dir" --check --json)"
  printf '%s\n' "$sync_output"
  assert_sync_ok "$sync_output"

  mkdir -p "$init_dir"
  "$tool" init --path "$init_dir"
  validate_output="$("$tool" validate --path "$init_dir" --json)"
  printf '%s\n' "$validate_output"
  assert_validate_with_mode "$validate_output" "placeholder"
  inspect_output="$("$tool" inspect --path "$init_dir" --json)"
  printf '%s\n' "$inspect_output"
  assert_inspect_current "$inspect_output" "Ossplate" "ossplate" "ossplate" "ossplate" "ossplate"
  sync_output="$("$tool" sync --path "$init_dir" --check --json)"
  printf '%s\n' "$sync_output"
  assert_sync_ok "$sync_output"

  "$tool" create "$custom_dir" \
    --name "blade-live" \
    --description "Live E2E custom identity fixture." \
    --repository "https://example.com/blade-live" \
    --license "Apache-2.0" \
    --author-name "Live E2E" \
    --author-email "live-e2e@example.com" \
    --rust-crate "blade_live" \
    --npm-package "blade-live" \
    --python-package "blade_live" \
    --command "blade-live"
  validate_output="$("$tool" validate --path "$custom_dir" --json)"
  printf '%s\n' "$validate_output"
  assert_validate_with_mode "$validate_output" "clean"
  inspect_output="$("$tool" inspect --path "$custom_dir" --json)"
  printf '%s\n' "$inspect_output"
  assert_inspect_current "$inspect_output" "blade-live" "blade_live" "blade-live" "blade_live" "blade-live"
  sync_output="$("$tool" sync --path "$custom_dir" --check --json)"
  printf '%s\n' "$sync_output"
  assert_sync_ok "$sync_output"

  run_generated_repo_verify "$custom_dir"
  run_scoped_identity_flow "$tool" "$target_root"
  run_upgrade_flow "$tool" "$target_root"
}

run_cargo_flow() {
  local cargo_root="$WORK_DIR/cargo"
  local install_root="$cargo_root/root"
  local bin_path="$install_root/bin/ossplate"
  mkdir -p "$cargo_root"

  if [[ "$OSTYPE" == msys* || "$OSTYPE" == cygwin* || "$(uname -s)" == MINGW* ]]; then
    bin_path="$install_root/bin/ossplate.exe"
  fi

  run_step "cargo:install" cargo install --root "$install_root" --force $CARGO_INSTALL_SPEC
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

  if [[ -n "$NPM_RUNTIME_SPEC" ]]; then
    run_step "npm:install" bash -lc "cd \"$npm_root\" && npm install \"$NPM_RUNTIME_SPEC\" \"$NPM_PACKAGE_SPEC\""
  else
    run_step "npm:install" bash -lc "cd \"$npm_root\" && npm install \"$NPM_PACKAGE_SPEC\""
  fi
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
  local python_path="$venv_dir/bin/python"
  local tool_path="$venv_dir/bin/ossplate"
  mkdir -p "$py_root"

  if [[ "$OSTYPE" == msys* || "$OSTYPE" == cygwin* || "$(uname -s)" == MINGW* ]]; then
    python_path="$venv_dir/Scripts/python.exe"
    tool_path="$venv_dir/Scripts/ossplate.exe"
  fi

  run_step "python:venv" "$PYTHON_BIN" -m venv "$venv_dir"
  run_step "python:install" "$python_path" -m pip install --upgrade pip
  run_step "python:install-package" "$python_path" -m pip install "$PYTHON_PACKAGE_SPEC"
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
