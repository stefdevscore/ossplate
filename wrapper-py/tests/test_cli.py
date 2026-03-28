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

from ossplate.cli import cli, get_packaged_binary_path


class CliTests(unittest.TestCase):
    supported_targets = (
        ("Darwin", "arm64", "darwin-arm64", "ossplate"),
        ("Darwin", "x86_64", "darwin-x64", "ossplate"),
        ("Linux", "x86_64", "linux-x64", "ossplate"),
        ("Windows", "AMD64", "win32-x64", "ossplate.exe"),
    )

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

    def test_packaged_binary_path_resolves_every_declared_target(self) -> None:
        base_dir = pathlib.Path(__file__).resolve().parents[1] / "src" / "ossplate"
        for system, machine, target, executable in self.supported_targets:
            expected = base_dir / "bin" / target / executable
            with self.subTest(system=system, machine=machine):
                with mock.patch("platform.system", return_value=system), mock.patch(
                    "platform.machine", return_value=machine
                ):
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
        core_binary = self.repo_root / "core-rs" / "target" / "debug" / "ossplate"
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
        self.assertNotIn("py3-none-any", wheel.name)
        self.assert_wheel_contents(wheel, self.current_target()[0])
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
