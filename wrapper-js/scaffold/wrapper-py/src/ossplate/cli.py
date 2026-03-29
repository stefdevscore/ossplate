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


def load_runtime_targets() -> list[dict]:
    scaffold_manifest = resources.files("ossplate").joinpath("scaffold", "runtime-targets.json")
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
    base_dir = base_dir or Path(resources.files("ossplate"))
    env_override = os.environ.get(ENV_OVERRIDE)
    if env_override:
        return env_override

    target = resolve_host_runtime_target(load_runtime_targets())
    folder = target["target"]
    executable = target["binary"]
    binary_path = base_dir / "bin" / folder / executable
    if not binary_path.exists():
        raise RuntimeError(f"Bundled ossplate binary not found at {binary_path}")
    return str(binary_path)


def get_binary_path() -> str:
    return get_packaged_binary_path()


def default_template_root() -> Path:
    return Path(resources.files("ossplate")) / "scaffold"


def build_cli_env(env: dict[str, str] | None = None) -> dict[str, str]:
    resolved = dict(env or os.environ.copy())
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
