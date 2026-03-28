#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import path from "node:path";

const repoRoot = path.resolve(import.meta.dirname, "..");
const result = spawnSync(
  "cargo",
  ["run", "--quiet", "--manifest-path", "core-rs/Cargo.toml", "--", "validate"],
  {
    cwd: repoRoot,
    stdio: "inherit"
  }
);

process.exit(result.status ?? 1);
