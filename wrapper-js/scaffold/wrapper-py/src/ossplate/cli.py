from __future__ import annotations

import os
import platform
import subprocess
import sys
from importlib import resources
from pathlib import Path

ENV_OVERRIDE = "OSSPLATE_BINARY"
TEMPLATE_ROOT_ENV = "OSSPLATE_TEMPLATE_ROOT"
TARGETS = {
    ("Darwin", "arm64"): ("darwin-arm64", "ossplate"),
    ("Darwin", "x86_64"): ("darwin-x64", "ossplate"),
    ("Linux", "x86_64"): ("linux-x64", "ossplate"),
    ("Windows", "AMD64"): ("win32-x64", "ossplate.exe"),
}


def get_packaged_binary_path(base_dir: Path | None = None) -> str:
    base_dir = base_dir or Path(resources.files("ossplate"))
    env_override = os.environ.get(ENV_OVERRIDE)
    if env_override:
        return env_override

    system = platform.system()
    machine = platform.machine()
    target = TARGETS.get((system, machine))
    if target is None:
        raise RuntimeError(f"Unsupported platform/arch: {system}/{machine}")

    folder, executable = target
    binary_path = base_dir / "bin" / folder / executable
    if not binary_path.exists():
        raise RuntimeError(f"Bundled ossplate binary not found at {binary_path}")
    return str(binary_path)


def get_binary_path() -> str:
    return get_packaged_binary_path()


def cli(args: tuple[str, ...]) -> int:
    env = os.environ.copy()
    env.setdefault(TEMPLATE_ROOT_ENV, str(Path(resources.files("ossplate")) / "scaffold"))
    result = subprocess.run([get_binary_path(), *args], check=False, env=env)
    return result.returncode


def main() -> None:
    raise SystemExit(cli(tuple(sys.argv[1:])))


if __name__ == "__main__":
    main()
