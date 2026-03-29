from __future__ import annotations

import os
import platform
import subprocess
import sysconfig
from pathlib import Path

try:
    from hatchling.builders.hooks.plugin.interface import BuildHookInterface
except ModuleNotFoundError:  # pragma: no cover - exercised in local unit tests without hatchling
    class BuildHookInterface:  # type: ignore[override]
        def __init__(self, *args, **kwargs) -> None:
            pass

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
RUNTIME_PACKAGES = {
    "darwin-arm64": "ossplate-darwin-arm64",
    "darwin-x64": "ossplate-darwin-x64",
    "linux-x64": "ossplate-linux-x64",
    "win32-x64": "ossplate-win32-x64",
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
        binary_source = (
            repo_root
            / "wrapper-js"
            / "platform-packages"
            / RUNTIME_PACKAGES[target]
            / "bin"
            / binary_name
        )
        if not binary_source.exists():
            raise RuntimeError(
                f"required ossplate binary for target {target} is missing at {binary_source}"
            )

        build_data["pure_python"] = False
        build_data["tag"] = f"py3-none-{platform_tag_for_target(target)}"
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


def platform_tag_for_target(target: str) -> str:
    if target == "linux-x64":
        return linux_platform_tag()
    if target == "darwin-arm64":
        return macos_platform_tag("arm64")
    if target == "darwin-x64":
        return macos_platform_tag("x86_64")
    if target == "win32-x64":
        return "win_amd64"
    raise RuntimeError(f"unsupported target for wheel tag generation: {target}")


def linux_platform_tag() -> str:
    libc_name, libc_version = platform.libc_ver()
    if libc_name != "glibc" or not libc_version:
        raise RuntimeError("linux wheel builds require glibc to derive a manylinux platform tag")

    major, minor, *_rest = libc_version.split(".")
    return f"manylinux_{major}_{minor}_x86_64"


def macos_platform_tag(arch: str) -> str:
    tag = sysconfig.get_platform().replace("-", "_").replace(".", "_")
    parts = tag.split("_")
    if len(parts) < 3 or parts[0] != "macosx":
        raise RuntimeError(f"unsupported macOS platform tag format: {tag}")

    major = int(parts[1])
    minor = int(parts[2])
    if arch == "arm64" and (major, minor) < (11, 0):
        major, minor = 11, 0

    return f"macosx_{major}_{minor}_{arch}"
