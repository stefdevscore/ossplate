from __future__ import annotations

import subprocess
from pathlib import Path

from hatchling.builders.hooks.plugin.interface import BuildHookInterface


class CustomBuildHook(BuildHookInterface):
    def initialize(self, version: str, build_data: dict) -> None:
        repo_root = Path(self.root).resolve().parent
        script = repo_root / "scripts" / "stage-distribution-assets.mjs"
        try:
            subprocess.run(["node", str(script)], cwd=repo_root, check=True)
        except FileNotFoundError as error:
            raise RuntimeError("node is required to stage distribution assets for wrapper-py builds") from error
