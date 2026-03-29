from __future__ import annotations

import os
import pathlib
import json
import subprocess
import sys
import platform
import shutil
import tempfile
import unittest
import zipfile
from unittest import mock

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))
import hatch_build

from ossplate.cli import cli, get_packaged_binary_path


class CliTests(unittest.TestCase):
    supported_targets = (
        ("Darwin", "arm64", "darwin-arm64", "ossplate"),
        ("Darwin", "x86_64", "darwin-x64", "ossplate"),
        ("Linux", "x86_64", "linux-x64", "ossplate"),
        ("Windows", "AMD64", "win32-x64", "ossplate.exe"),
    )
    wheel_size_budgets = {
        "darwin-arm64": (4_000_000, 10_000_000),
        "darwin-x64": (8_000_000, 18_000_000),
        "linux-x64": (12_000_000, 40_000_000),
        "win32-x64": (12_000_000, 40_000_000),
    }

    def setUp(self) -> None:
        self.fixture_dir = tempfile.TemporaryDirectory()
        self.fixture = self.create_stub_binary(pathlib.Path(self.fixture_dir.name))
        self.repo_root = pathlib.Path(__file__).resolve().parents[2]
        self.scaffold_manifest = json.loads(
            (self.repo_root / "scaffold-manifest.json").read_text()
        )

    def tearDown(self) -> None:
        os.environ.pop("OSSPLATE_BINARY", None)
        os.environ.pop("OSSPLATE_PY_TARGET", None)
        self.fixture_dir.cleanup()

    def test_env_override_takes_precedence(self) -> None:
        os.environ["OSSPLATE_BINARY"] = self.fixture
        self.assertEqual(get_packaged_binary_path(), self.fixture)

    def test_packaged_binary_path_resolves_host_target(self) -> None:
        base_dir = pathlib.Path(__file__).resolve().parents[1] / "src" / "ossplate"
        target, executable = self.current_target()
        expected = base_dir / "bin" / target / executable
        self.assertTrue(expected.exists(), f"expected host binary at {expected}")
        self.assertEqual(get_packaged_binary_path(base_dir), str(expected))

    def test_unsupported_platform_fails(self) -> None:
        with mock.patch("platform.system", return_value="Linux"), mock.patch(
            "platform.machine", return_value="arm64"
        ):
            with self.assertRaisesRegex(RuntimeError, "Unsupported platform/arch"):
                get_packaged_binary_path(pathlib.Path(__file__).parent)

    def test_cli_forwards_arguments_to_binary(self) -> None:
        os.environ["OSSPLATE_BINARY"] = self.fixture
        result = cli(("version",))
        self.assertEqual(result, 0)

    def test_python_wrapper_matches_rust_contract_via_env_override(self) -> None:
        _, host_executable = self.current_target()
        core_binary = (
            self.repo_root / "core-rs" / "target" / "debug" / host_executable
        )
        repo_root = self.repo_root
        subprocess.run(["cargo", "build"], cwd=core_binary.parents[2], check=True)
        for args in (
            ("version",),
            ("validate", "--path", str(repo_root), "--json"),
            ("sync", "--path", str(repo_root), "--check"),
        ):
            direct = subprocess.run(
                [str(core_binary), *args],
                check=True,
                capture_output=True,
                text=True,
            )
            os.environ["OSSPLATE_BINARY"] = str(core_binary)
            wrapped = subprocess.run(
                [sys.executable, "-m", "ossplate.cli", *args],
                check=True,
                capture_output=True,
                text=True,
                env=os.environ.copy(),
            )
            self.assertEqual(wrapped.stdout.strip(), direct.stdout.strip())

    def test_packaged_python_wrapper_can_create_from_scaffold_payload(self) -> None:
        repo_root = self.repo_root
        build_venv_dir = repo_root / "wrapper-py" / ".tmp-build-venv"
        dist_dir = repo_root / "wrapper-py" / "dist"
        shutil.rmtree(build_venv_dir, ignore_errors=True)
        shutil.rmtree(dist_dir, ignore_errors=True)
        subprocess.run(["cargo", "build"], cwd=repo_root / "core-rs", check=True)
        subprocess.run([sys.executable, "-m", "venv", str(build_venv_dir)], check=True)
        build_python = self.venv_executable(build_venv_dir, "python")
        subprocess.run(
            [str(build_python), "-m", "pip", "install", "build"],
            check=True,
            stdout=subprocess.DEVNULL,
        )
        os.environ["OSSPLATE_PY_TARGET"] = self.current_target()[0]
        subprocess.run(
            [str(build_python), "-m", "build", "--wheel"],
            cwd=repo_root / "wrapper-py",
            check=True,
            env=os.environ.copy(),
        )
        wheels = sorted(dist_dir.glob("ossplate-*.whl"))
        self.assertEqual(len(wheels), 1, f"expected one built wheel in {dist_dir}")
        wheel = wheels[0]
        self.assertIn("py3-none-", wheel.name)
        self.assertNotIn("py3-none-any", wheel.name)
        self.assertNotIn("-cp", wheel.name)
        self.assert_wheel_contents(wheel, self.current_target()[0])
        self.assert_wheel_size_budget(wheel, self.current_target()[0])
        venv_dir = repo_root / "wrapper-py" / ".tmp-wheel-venv"
        target_dir = repo_root / "wrapper-py" / ".tmp-wheel-created"
        shutil.rmtree(venv_dir, ignore_errors=True)
        shutil.rmtree(target_dir, ignore_errors=True)
        subprocess.run([sys.executable, "-m", "venv", str(venv_dir)], check=True)
        pip = self.venv_executable(venv_dir, "pip")
        tool = self.venv_executable(venv_dir, "ossplate")
        subprocess.run([str(pip), "install", str(wheel)], check=True, stdout=subprocess.DEVNULL)
        _, host_executable = self.current_target()
        direct_version = subprocess.run(
            [str(repo_root / "core-rs" / "target" / "debug" / host_executable), "version"],
            check=True,
            capture_output=True,
            text=True,
        )
        packaged_version = subprocess.run(
            [str(tool), "version"],
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertEqual(packaged_version.stdout.strip(), direct_version.stdout.strip())
        subprocess.run([str(tool), "create", str(target_dir)], check=True)
        output = subprocess.run(
            [str(tool), "validate", "--path", str(target_dir), "--json"],
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertEqual(output.stdout.strip(), '{"ok":true,"issues":[]}')
        shutil.rmtree(build_venv_dir, ignore_errors=True)
        shutil.rmtree(venv_dir, ignore_errors=True)
        shutil.rmtree(target_dir, ignore_errors=True)

    def test_linux_wheel_tags_use_manylinux(self) -> None:
        with mock.patch("platform.libc_ver", return_value=("glibc", "2.39")):
            self.assertEqual(
                hatch_build.platform_tag_for_target("linux-x64"),
                "manylinux_2_39_x86_64",
            )

    def test_macos_x64_wheel_tag_drops_universal2(self) -> None:
        with mock.patch("sysconfig.get_platform", return_value="macosx-10.9-universal2"):
            self.assertEqual(
                hatch_build.platform_tag_for_target("darwin-x64"),
                "macosx_10_9_x86_64",
            )

    def test_macos_arm64_wheel_tag_targets_arm64(self) -> None:
        with mock.patch("sysconfig.get_platform", return_value="macosx-15.0-arm64"):
            self.assertEqual(
                hatch_build.platform_tag_for_target("darwin-arm64"),
                "macosx_15_0_arm64",
            )

    def test_macos_arm64_wheel_tag_uses_minimum_supported_version(self) -> None:
        with mock.patch("sysconfig.get_platform", return_value="macosx-10.9-universal2"):
            self.assertEqual(
                hatch_build.platform_tag_for_target("darwin-arm64"),
                "macosx_11_0_arm64",
            )

    def assert_wheel_contents(self, wheel: pathlib.Path, target: str) -> None:
        with zipfile.ZipFile(wheel) as archive:
            names = archive.namelist()

        packaged_binaries = sorted(
            name for name in names if name.startswith("ossplate/bin/") and not name.endswith("/")
        )
        expected_binary = f"ossplate/bin/{target}/{self.target_executable(target)}"
        self.assertEqual(packaged_binaries, [expected_binary])

        prefix = "ossplate/scaffold/"
        for relative_path in self.scaffold_manifest["requiredPaths"]:
            self.assertIn(
                f"{prefix}{relative_path}",
                names,
                f"expected wheel scaffold file {relative_path}",
            )

        for excluded_prefix in self.scaffold_manifest["excludedPrefixes"]:
            self.assertFalse(
                any(name.startswith(f"{prefix}{excluded_prefix}") for name in names),
                f"unexpected wheel scaffold file under {excluded_prefix}",
            )

    def assert_wheel_size_budget(self, wheel: pathlib.Path, target: str) -> None:
        compressed_budget, unpacked_budget = self.wheel_size_budgets[target]
        compressed_size = wheel.stat().st_size
        with zipfile.ZipFile(wheel) as archive:
            unpacked_size = sum(info.file_size for info in archive.infolist())

        self.assertLessEqual(
            compressed_size,
            compressed_budget,
            f"{wheel.name} compressed size {compressed_size} exceeds budget {compressed_budget}",
        )
        self.assertLessEqual(
            unpacked_size,
            unpacked_budget,
            f"{wheel.name} unpacked size {unpacked_size} exceeds budget {unpacked_budget}",
        )

    def create_stub_binary(self, directory: pathlib.Path) -> str:
        if os.name == "nt":
            path = directory / "ossplate-stub.cmd"
            path.write_text("@echo {\"tool\":\"stub-tool\",\"version\":\"9.9.9\"}\r\n")
            return str(path)

        path = directory / "ossplate-stub.sh"
        path.write_text("#!/usr/bin/env sh\necho '{\"tool\":\"stub-tool\",\"version\":\"9.9.9\"}'\n")
        path.chmod(0o755)
        return str(path)

    def current_target(self) -> tuple[str, str]:
        machine = platform.machine()
        if (platform.system(), machine) == ("Windows", "x86_64"):
            machine = "AMD64"

        for system, host_machine, target, executable in self.supported_targets:
            if (system, host_machine) == (platform.system(), machine):
                return target, executable
        raise AssertionError(f"unsupported host for test wheel build: {platform.system()}/{machine}")

    def target_executable(self, target: str) -> str:
        for _, _, supported_target, executable in self.supported_targets:
            if supported_target == target:
                return executable
        raise AssertionError(f"unknown target {target}")

    def venv_executable(self, venv_dir: pathlib.Path, name: str) -> pathlib.Path:
        scripts_dir = "Scripts" if os.name == "nt" else "bin"
        suffix = ".exe" if os.name == "nt" else ""
        return venv_dir / scripts_dir / f"{name}{suffix}"


if __name__ == "__main__":
    unittest.main()
