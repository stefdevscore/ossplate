import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

import { parseArgs, runPublish } from "./publish-local.mjs";

test("parseArgs reads publish options", () => {
  const parsed = parseArgs([
    "--root",
    "/tmp/ossplate",
    "--dry-run",
    "--registry",
    "pypi",
    "--skip-existing"
  ]);
  assert.deepEqual(parsed, {
    root: "/tmp/ossplate",
    dryRun: true,
    registry: "pypi",
    skipExisting: true
  });
});

test("runPublish executes dry-run registries in the expected order", () => {
  const root = makeFixtureRoot();
  const context = makeFakeContext();

  runPublish(
    {
      root,
      dryRun: true,
      registry: "all",
      skipExisting: false
    },
    context
  );

  assert.deepEqual(
    context.labels(),
    [
      "tool:validate",
      "tool:sync-check",
      "release:assert",
      "js:lockfile-assert",
      "publish:assert",
      "npm:build-core",
      "npm:runtime:publish",
      "npm:install-build-deps",
      "npm:build",
      "npm:top-level:publish",
      "pypi:build-core",
      "pypi:install-build-tools",
      "pypi:build-wheel",
      "pypi:build-sdist",
      "pypi:check",
      "cargo:publish"
    ]
  );
});

test("runPublish stops on first registry failure and never reaches later registries", () => {
  const root = makeFixtureRoot();
  const context = makeFakeContext({ failOnLabel: "npm:top-level:publish" });

  assert.throws(
    () =>
      runPublish(
        {
          root,
          dryRun: true,
          registry: "all",
          skipExisting: false
        },
        context
      ),
    /partial publish completed before failure: npm/
  );

  assert.equal(context.labels().includes("pypi:build-core"), false);
  assert.equal(context.labels().includes("cargo:publish"), false);
});

test("runPublish short-circuits npm before publish when non-host runtimes are unavailable", () => {
  const root = makeFixtureRoot();
  const context = makeFakeContext({
    npmVersions: new Set(["ossplate-linux-x64@0.1.22"])
  });

  assert.throws(
    () =>
      runPublish(
        {
          root,
          dryRun: false,
          registry: "npm",
          skipExisting: false
        },
        context
      ),
    /publish them from matching host runners first/
  );

  assert.deepEqual(context.labels(), [
    "tool:validate",
    "tool:sync-check",
    "release:assert",
    "js:lockfile-assert",
    "publish:assert"
  ]);
});

test("runPublish never invokes git or release mutation commands", () => {
  const root = makeFixtureRoot();
  const context = makeFakeContext();

  runPublish(
    {
      root,
      dryRun: true,
      registry: "cargo",
      skipExisting: false
    },
    context
  );

  for (const command of context.commands()) {
    assert.notEqual(command.program, "git");
    assert.equal(command.args.some((arg) => String(arg).includes("release.yml")), false);
  }
});

function makeFixtureRoot() {
  const root = mkdtempSync(join(tmpdir(), "ossplate-publish-local-"));
  mkdirSync(join(root, "core-rs"), { recursive: true });
  mkdirSync(join(root, "scripts"), { recursive: true });
  mkdirSync(join(root, "wrapper-js", "platform-packages", "ossplate-darwin-arm64"), {
    recursive: true
  });
  mkdirSync(join(root, "wrapper-js", "platform-packages", "ossplate-darwin-x64"), {
    recursive: true
  });
  mkdirSync(join(root, "wrapper-js", "platform-packages", "ossplate-linux-x64"), {
    recursive: true
  });
  mkdirSync(join(root, "wrapper-js", "platform-packages", "ossplate-win32-x64"), {
    recursive: true
  });
  mkdirSync(join(root, "wrapper-py", "dist", "linux-x64"), { recursive: true });
  mkdirSync(join(root, "wrapper-py", "dist", "sdist"), { recursive: true });

  writeFileSync(
    join(root, "core-rs", "Cargo.toml"),
    `[package]
name = "ossplate"
version = "0.1.22"
`
  );
  writeFileSync(join(root, "scripts", "assert-release-state.mjs"), "console.log('ok')\n");
  writeFileSync(join(root, "scripts", "assert-js-lockfile-state.mjs"), "console.log('ok')\n");
  writeFileSync(join(root, "scripts", "assert-publish-readiness.mjs"), "console.log('ok')\n");
  writeFileSync(join(root, "scripts", "wait-for-npm-versions.mjs"), "console.log('ok')\n");
  writeFileSync(
    join(root, "wrapper-js", "package.json"),
    JSON.stringify(
      {
        name: "ossplate",
        version: "0.1.22",
        optionalDependencies: {
          "ossplate-darwin-arm64": "0.1.22",
          "ossplate-darwin-x64": "0.1.22",
          "ossplate-linux-x64": "0.1.22",
          "ossplate-windows-x64": "0.1.22"
        }
      },
      null,
      2
    )
  );
  writeFileSync(join(root, "wrapper-py", "dist", "linux-x64", "ossplate.whl"), "wheel");
  writeFileSync(join(root, "wrapper-py", "dist", "sdist", "ossplate.tar.gz"), "sdist");
  return root;
}

function makeFakeContext({ failOnLabel = null, npmVersions = new Set(), cargoVersions = new Set() } = {}) {
  const executed = [];
  return {
    run(command) {
      executed.push(command);
      if (command.label === failOnLabel) {
        throw new Error(`forced failure for ${command.label}`);
      }
    },
    npmVersionExists(name, version) {
      return npmVersions.has(`${name}@${version}`);
    },
    cargoVersionExists(name, version) {
      return cargoVersions.has(`${name}@${version}`);
    },
    pythonCommand() {
      return "python3";
    },
    platform() {
      return "linux";
    },
    arch() {
      return "x64";
    },
    log() {},
    labels() {
      return executed.map((command) => command.label);
    },
    commands() {
      return executed;
    }
  };
}
