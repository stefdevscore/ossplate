from __future__ import annotations

import json
import os
import platform
import subprocess
import sys
from importlib import resources
from pathlib import Path

ENV_OVERRIDE = "OSSPLATE_BINARY"
TEMPLATE_ROOT_ENV = "OSSPLATE_TEMPLATE_ROOT"
FORWARDED_ENV_KEYS = (
    "ALL_PROXY",
    "APPDATA",
    "CARGO_HOME",
    "CARGO_REGISTRY_TOKEN",
    "CI",
    "COLORTERM",
    "ComSpec",
    "GIT_ASKPASS",
    "HOME",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    "LOCALAPPDATA",
    "NO_COLOR",
    "NO_PROXY",
    "NPM_TOKEN",
    "PATH",
    "PATHEXT",
    "PROGRAMDATA",
    "PYENV_ROOT",
    "RUSTUP_HOME",
    "SSL_CERT_DIR",
    "SSL_CERT_FILE",
    "SYSTEMROOT",
    "SystemRoot",
    "TEMP",
    "TERM",
    "TMP",
    "TMPDIR",
    "TWINE_PASSWORD",
    "TWINE_USERNAME",
    "USERPROFILE",
    "VIRTUAL_ENV",
    "XDG_CACHE_HOME",
    "XDG_CONFIG_HOME",
    "XDG_DATA_HOME",
    "XDG_RUNTIME_DIR",
)

PACKAGE_NAME = __package__ or __name__.split(".")[0]


def load_runtime_targets() -> list[dict]:
    scaffold_manifest = resources.files(PACKAGE_NAME).joinpath("scaffold", "runtime-targets.json")
    if scaffold_manifest.is_file():
        return json.loads(scaffold_manifest.read_text(encoding="utf-8"))["targets"]

    source_manifest = Path(__file__).resolve().parents[3] / "runtime-targets.json"
    return json.loads(source_manifest.read_text(encoding="utf-8"))["targets"]


def resolve_host_runtime_target(runtime_targets: list[dict]) -> dict:
    system = platform.system()
    machine = platform.machine()
    target = next(
        (
            entry
            for entry in runtime_targets
            if entry["python"]["system"] == system and machine in entry["python"]["machines"]
        ),
        None,
    )
    if target is None:
        raise RuntimeError(f"Unsupported platform/arch: {system}/{machine}")
    return target


def get_packaged_binary_path(base_dir: Path | None = None) -> str:
    base_dir = base_dir or Path(resources.files(PACKAGE_NAME))
    env_override = os.environ.get(ENV_OVERRIDE)
    if env_override:
        return env_override

    target = resolve_host_runtime_target(load_runtime_targets())
    folder = target["target"]
    executable = target["binary"]
    binary_path = base_dir / "bin" / folder / executable
    if not binary_path.exists():
        raise RuntimeError(f"Bundled CLI binary not found at {binary_path}")
    return str(binary_path)


def get_binary_path() -> str:
    return get_packaged_binary_path()


def default_template_root() -> Path:
    return Path(resources.files(PACKAGE_NAME)) / "scaffold"


def build_cli_env(env: dict[str, str] | None = None) -> dict[str, str]:
    source_env = env or os.environ.copy()
    resolved = {
        key: source_env[key]
        for key in FORWARDED_ENV_KEYS
        if key in source_env
    }
    for key, value in source_env.items():
        if key.startswith("OSSPLATE_"):
            resolved[key] = value
    resolved.setdefault(TEMPLATE_ROOT_ENV, str(default_template_root()))
    return resolved


def run_binary(args: tuple[str, ...], env: dict[str, str] | None = None) -> int:
    result = subprocess.run([get_binary_path(), *args], check=False, env=build_cli_env(env))
    return result.returncode


def cli(args: tuple[str, ...]) -> int:
    return run_binary(args)


def main() -> None:
    raise SystemExit(cli(tuple(sys.argv[1:])))


if __name__ == "__main__":
    main()
