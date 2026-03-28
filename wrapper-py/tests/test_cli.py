from __future__ import annotations

import os
import pathlib
import json
import subprocess
import sys
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
        self.fixture = str(
            pathlib.Path(__file__).with_name("fixtures").joinpath("ossplate-stub.sh")
        )
        self.repo_root = pathlib.Path(__file__).resolve().parents[2]
        self.scaffold_manifest = json.loads(
            (self.repo_root / "scaffold-manifest.json").read_text()
        )

    def tearDown(self) -> None:
        os.environ.pop("OSSPLATE_BINARY", None)

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
        subprocess.run(["rm", "-rf", str(build_venv_dir)], check=True)
        subprocess.run(["cargo", "build"], cwd=repo_root / "core-rs", check=True)
        subprocess.run([sys.executable, "-m", "venv", str(build_venv_dir)], check=True)
        build_python = build_venv_dir / "bin" / "python"
        subprocess.run(
            [str(build_python), "-m", "pip", "install", "build"],
            check=True,
            stdout=subprocess.DEVNULL,
        )
        subprocess.run(
            [str(build_python), "-m", "build", "--wheel"],
            cwd=repo_root / "wrapper-py",
            check=True,
        )
        wheel = next((repo_root / "wrapper-py" / "dist").glob("ossplate-*.whl"))
        self.assert_wheel_contents(wheel)
        venv_dir = repo_root / "wrapper-py" / ".tmp-wheel-venv"
        target_dir = repo_root / "wrapper-py" / ".tmp-wheel-created"
        subprocess.run(["rm", "-rf", str(venv_dir), str(target_dir)], check=True)
        subprocess.run([sys.executable, "-m", "venv", str(venv_dir)], check=True)
        pip = venv_dir / "bin" / "pip"
        tool = venv_dir / "bin" / "ossplate"
        subprocess.run([str(pip), "install", str(wheel)], check=True, stdout=subprocess.DEVNULL)
        direct_version = subprocess.run(
            [str(repo_root / "core-rs" / "target" / "debug" / "ossplate"), "version"],
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
        subprocess.run(
            ["rm", "-rf", str(build_venv_dir), str(venv_dir), str(target_dir)],
            check=True,
        )

    def assert_wheel_contents(self, wheel: pathlib.Path) -> None:
        with zipfile.ZipFile(wheel) as archive:
            names = archive.namelist()

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


if __name__ == "__main__":
    unittest.main()
