import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import path from "node:path";

const repoRoot = path.resolve(import.meta.dirname, "..");

test("rust-backed validator passes on the current repo", () => {
  const output = execFileSync(
    "cargo",
    ["run", "--quiet", "--manifest-path", "core-rs/Cargo.toml", "--", "validate", "--json"],
    { cwd: repoRoot, encoding: "utf8" }
  ).trim();
  const parsed = JSON.parse(output);
  assert.equal(parsed.ok, true);
  assert.deepEqual(parsed.issues, []);
});

test("sync --check passes on the current repo", () => {
  execFileSync(
    "cargo",
    ["run", "--quiet", "--manifest-path", "core-rs/Cargo.toml", "--", "sync", "--check"],
    { cwd: repoRoot, stdio: "pipe" }
  );
});
