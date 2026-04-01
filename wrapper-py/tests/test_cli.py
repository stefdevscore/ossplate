from __future__ import annotations

import os
import pathlib
import json
import subprocess
import sys
import platform
import importlib
import shutil
import tempfile
import unittest
import zipfile
from unittest import mock

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1] / "src"))
import hatch_build

WRAPPER_ROOT = pathlib.Path(__file__).resolve().parents[1]
REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]
PYPROJECT_TEXT = (WRAPPER_ROOT / "pyproject.toml").read_text()
PYTHON_PACKAGE_NAME = next(
    line.split("=", 1)[1].strip().strip('"')
    for line in PYPROJECT_TEXT.splitlines()
    if line.startswith("name = ")
)
PYTHON_MODULE_NAME = PYTHON_PACKAGE_NAME.replace("-", "_").replace(".", "_")
WRAPPER_COMMAND = next(
    line.split("=", 1)[0].strip()
    for line in PYPROJECT_TEXT.splitlines()
    if ".cli:main" in line
)
TEST_PYTHON = next(
    candidate
    for candidate in ("python3.14", "python3.13", "python3.12", "python3.11", "python3.10", "python3")
    if shutil.which(candidate)
    and subprocess.run(
        [candidate, "-c", "import sys; raise SystemExit(0 if sys.version_info >= (3, 10) else 1)"],
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    ).returncode
    == 0
)
CLI_MODULE = importlib.import_module(f"{PYTHON_MODULE_NAME}.cli")
build_cli_env = CLI_MODULE.build_cli_env
cli = CLI_MODULE.cli
get_packaged_binary_path = CLI_MODULE.get_packaged_binary_path


class CliTests(unittest.TestCase):
    supported_targets = tuple(
        (
            target["python"]["system"],
            target["python"]["machines"][0],
            target["target"],
            target["binary"],
        )
        for target in json.loads((REPO_ROOT / "runtime-targets.json").read_text())["targets"]
    )
    wheel_size_budgets = {
        "darwin-arm64": (4_000_000, 10_250_000),
        "darwin-x64": (8_000_000, 18_000_000),
        "linux-x64": (12_000_000, 40_000_000),
        "win32-x64": (12_000_000, 40_000_000),
    }

    @classmethod
    def setUpClass(cls) -> None:
        subprocess.run(
            ["node", str(REPO_ROOT / "scripts" / "stage-distribution-assets.mjs"), "embedded-template"],
            cwd=REPO_ROOT,
            check=True,
            stdout=subprocess.DEVNULL,
        )
        subprocess.run(["cargo", "build"], cwd=REPO_ROOT / "core-rs", check=True)
        subprocess.run(
            ["node", str(REPO_ROOT / "scripts" / "stage-distribution-assets.mjs")],
            cwd=REPO_ROOT,
            check=True,
            stdout=subprocess.DEVNULL,
        )

    def setUp(self) -> None:
        self.fixture_dir = tempfile.TemporaryDirectory()
        self.fixture = self.create_stub_binary(pathlib.Path(self.fixture_dir.name))
        self.repo_root = REPO_ROOT
        self.scaffold_manifest = json.loads(
            (self.repo_root / "scaffold-payload.json").read_text()
        )

    def tearDown(self) -> None:
        os.environ.pop("OSSPLATE_BINARY", None)
        os.environ.pop("OSSPLATE_PY_TARGET", None)
        self.fixture_dir.cleanup()

    def test_env_override_takes_precedence(self) -> None:
        os.environ["OSSPLATE_BINARY"] = self.fixture
        self.assertEqual(get_packaged_binary_path(), self.fixture)

    def test_packaged_binary_path_resolves_host_target(self) -> None:
        base_dir = WRAPPER_ROOT / "src" / PYTHON_MODULE_NAME
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

    def test_build_cli_env_forwards_only_wrapper_contract(self) -> None:
        env = build_cli_env(
            {
                "PATH": "/usr/bin",
                "HOME": "/tmp/home",
                "NPM_TOKEN": "npm-secret",
                "OSSPLATE_NPM_WAIT_ATTEMPTS": "12",
                "OSSPLATE_TEMPLATE_ROOT": "/custom/scaffold",
                "AWS_SECRET_ACCESS_KEY": "should-not-forward",
            }
        )
        self.assertEqual(env["PATH"], "/usr/bin")
        self.assertEqual(env["HOME"], "/tmp/home")
        self.assertEqual(env["NPM_TOKEN"], "npm-secret")
        self.assertEqual(env["OSSPLATE_NPM_WAIT_ATTEMPTS"], "12")
        self.assertEqual(env["OSSPLATE_TEMPLATE_ROOT"], "/custom/scaffold")
        self.assertNotIn("AWS_SECRET_ACCESS_KEY", env)

    def test_python_wrapper_matches_rust_contract_via_env_override(self) -> None:
        _, host_executable = self.current_target()
        core_binary = (
            self.repo_root / "core-rs" / "target" / "debug" / host_executable
        )
        repo_root = self.repo_root
        subprocess.run(
            ["node", str(REPO_ROOT / "scripts" / "stage-distribution-assets.mjs"), "embedded-template"],
            cwd=REPO_ROOT,
            check=True,
            stdout=subprocess.DEVNULL,
        )
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
                [sys.executable, "-m", f"{PYTHON_MODULE_NAME}.cli", *args],
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
        subprocess.run(
            [ "node", str(repo_root / "scripts" / "stage-distribution-assets.mjs"), "embedded-template"],
            cwd=repo_root,
            check=True,
            stdout=subprocess.DEVNULL,
        )
        subprocess.run(["cargo", "build"], cwd=repo_root / "core-rs", check=True)
        subprocess.run([TEST_PYTHON, "-m", "venv", str(build_venv_dir)], check=True)
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
        wheels = sorted(dist_dir.glob("*.whl"))
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
        subprocess.run([TEST_PYTHON, "-m", "venv", str(venv_dir)], check=True)
        pip = self.venv_executable(venv_dir, "pip")
        tool = self.venv_executable(venv_dir, WRAPPER_COMMAND)
        subprocess.run([str(pip), "install", str(wheel)], check=True, stdout=subprocess.DEVNULL)
        packaged_env = os.environ.copy()
        packaged_env.pop("OSSPLATE_TEMPLATE_ROOT", None)
        packaged_env.pop("PYTHONPATH", None)
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
            env=packaged_env,
        )
        self.assertEqual(packaged_version.stdout.strip(), direct_version.stdout.strip())
        subprocess.run([str(tool), "create", str(target_dir)], check=True, env=packaged_env)
        output = subprocess.run(
            [str(tool), "validate", "--path", str(target_dir), "--json"],
            check=True,
            capture_output=True,
            text=True,
            env=packaged_env,
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

    def test_staged_runtime_binary_path_uses_neutral_artifact_root(self) -> None:
        path = hatch_build.staged_runtime_binary_path(self.repo_root, "linux-x64")
        self.assertEqual(
            path,
            self.repo_root / ".dist-assets" / "runtime" / "linux-x64" / self.target_executable("linux-x64"),
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
            name
            for name in names
            if name.startswith(f"{PYTHON_MODULE_NAME}/bin/") and not name.endswith("/")
        )
        expected_binary = f"{PYTHON_MODULE_NAME}/bin/{target}/{self.target_executable(target)}"
        self.assertEqual(packaged_binaries, [expected_binary])

        prefix = f"{PYTHON_MODULE_NAME}/scaffold/"
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
            path = directory / f"{WRAPPER_COMMAND}-stub.cmd"
            path.write_text("@echo {\"tool\":\"stub-tool\",\"version\":\"9.9.9\"}\r\n")
            return str(path)

        path = directory / f"{WRAPPER_COMMAND}-stub.sh"
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
