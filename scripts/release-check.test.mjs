import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const cli = join(repoRoot, "scripts", "release-check.mjs");

test("release-check scaffold-assets preserves current output contract", () => {
  const output = execFileSync("node", [cli, "scaffold-assets"], {
    cwd: repoRoot,
    encoding: "utf8"
  }).trim();
  assert.equal(output, "scaffold assets ok");
});

test("release-check scaffold-mirrors stays as a compatibility alias", () => {
  const output = execFileSync("node", [cli, "scaffold-mirrors"], {
    cwd: repoRoot,
    encoding: "utf8"
  }).trim();
  assert.equal(output, "scaffold assets ok");
});

test("release-check package-cleanliness preserves the no-leak contract", () => {
  const output = execFileSync("node", [cli, "package-cleanliness"], {
    cwd: repoRoot,
    encoding: "utf8"
  }).trim();
  assert.equal(output, "package cleanliness ok");
});

test("release-check generated-project-dogfood exercises the generated project gate", () => {
  const output = execFileSync("node", [cli, "generated-project-dogfood"], {
    cwd: repoRoot,
    encoding: "utf8"
  }).trim();
  assert.equal(output, "generated project dogfood ok");
});

test("release-check rejects unknown subcommands", () => {
  assert.throws(
    () =>
      execFileSync("node", [cli, "unknown-command"], {
        cwd: repoRoot,
        encoding: "utf8",
        stdio: "pipe"
      }),
    /usage: node scripts\/release-check\.mjs/
  );
});
