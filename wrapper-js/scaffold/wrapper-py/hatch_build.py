from __future__ import annotations

import os
import platform
import subprocess
from pathlib import Path

from hatchling.builders.hooks.plugin.interface import BuildHookInterface

BUILD_TARGET_ENV = "OSSPLATE_PY_TARGET"
TARGETS = {
    "darwin-arm64": "ossplate",
    "darwin-x64": "ossplate",
    "linux-x64": "ossplate",
    "win32-x64": "ossplate.exe",
}
HOST_TARGETS = {
    ("Darwin", "arm64"): "darwin-arm64",
    ("Darwin", "x86_64"): "darwin-x64",
    ("Linux", "x86_64"): "linux-x64",
    ("Windows", "AMD64"): "win32-x64",
    ("Windows", "x86_64"): "win32-x64",
}


class CustomBuildHook(BuildHookInterface):
    def initialize(self, version: str, build_data: dict) -> None:
        repo_root = Path(self.root).resolve().parent
        script = repo_root / "scripts" / "stage-distribution-assets.mjs"
        try:
            subprocess.run(["node", str(script)], cwd=repo_root, check=True)
        except FileNotFoundError as error:
            raise RuntimeError("node is required to stage distribution assets for wrapper-py builds") from error

        target = resolve_build_target()
        binary_name = TARGETS[target]
        binary_source = repo_root / "wrapper-js" / "bin" / target / binary_name
        if not binary_source.exists():
            raise RuntimeError(
                f"required ossplate binary for target {target} is missing at {binary_source}"
            )

        build_data["pure_python"] = False
        build_data["infer_tag"] = True
        force_include = build_data.setdefault("force_include", {})
        force_include[str(binary_source)] = f"ossplate/bin/{target}/{binary_name}"


def resolve_build_target() -> str:
    target = os.environ.get(BUILD_TARGET_ENV)
    if target:
        if target not in TARGETS:
            raise RuntimeError(f"unsupported {BUILD_TARGET_ENV} value: {target}")
        return target

    host = HOST_TARGETS.get((platform.system(), platform.machine()))
    if host is None:
        raise RuntimeError(
            f"unsupported host platform for wrapper-py wheel build: {platform.system()}/{platform.machine()}"
        )
    return host
