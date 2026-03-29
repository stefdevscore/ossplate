from __future__ import annotations

import json
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


def load_runtime_targets(repo_root: Path) -> list[dict]:
    return json.loads((repo_root / "runtime-targets.json").read_text(encoding="utf-8"))["targets"]


class CustomBuildHook(BuildHookInterface):
    def initialize(self, version: str, build_data: dict) -> None:
        repo_root = Path(self.root).resolve().parent
        runtime_targets = load_runtime_targets(repo_root)
        script = repo_root / "scripts" / "stage-distribution-assets.mjs"
        try:
            subprocess.run(["node", str(script)], cwd=repo_root, check=True)
        except FileNotFoundError as error:
            raise RuntimeError("node is required to stage distribution assets for wrapper-py builds") from error

        target = resolve_build_target()
        target_spec = runtime_target_by_name(runtime_targets, target)
        binary_name = target_spec["binary"]
        binary_source = staged_runtime_binary_path(repo_root, target)
        if not binary_source.exists():
            raise RuntimeError(
                f"required ossplate binary for target {target} is missing at {binary_source}"
            )

        build_data["pure_python"] = False
        build_data["tag"] = f"py3-none-{platform_tag_for_target(target)}"
        force_include = build_data.setdefault("force_include", {})
        force_include[str(binary_source)] = f"ossplate/bin/{target}/{binary_name}"


def staged_runtime_binary_path(repo_root: Path, target: str) -> Path:
    binary_name = runtime_target_by_name(load_runtime_targets(repo_root), target)["binary"]
    return repo_root / ".dist-assets" / "runtime" / target / binary_name


def resolve_build_target() -> str:
    repo_root = Path(__file__).resolve().parent.parent
    runtime_targets = load_runtime_targets(repo_root)
    target = os.environ.get(BUILD_TARGET_ENV)
    if target:
        if not any(entry["target"] == target for entry in runtime_targets):
            raise RuntimeError(f"unsupported {BUILD_TARGET_ENV} value: {target}")
        return target

    host = next(
        (
            entry["target"]
            for entry in runtime_targets
            if entry["python"]["system"] == platform.system()
            and platform.machine() in entry["python"]["machines"]
        ),
        None,
    )
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


def runtime_target_by_name(runtime_targets: list[dict], target: str) -> dict:
    for entry in runtime_targets:
        if entry["target"] == target:
            return entry
    raise RuntimeError(f"unsupported runtime target: {target}")


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
