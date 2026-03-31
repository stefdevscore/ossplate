import test from "node:test";
import assert from "node:assert/strict";
import { existsSync, mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

import { buildCommandEnv, parseArgs, runPublish } from "./publish-local.mjs";
import { waitForNpmVersions } from "./wait-for-npm-versions.mjs";

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
  const context = makeFakeContext({
    onRun(command) {
      if (command.label === "pypi:build-wheel") {
        writeFileSync(join(root, "wrapper-py", "dist", "linux-x64", "fresh.whl"), "wheel");
      }
      if (command.label === "pypi:build-sdist") {
        writeFileSync(join(root, "wrapper-py", "dist", "sdist", "fresh.tar.gz"), "sdist");
      }
    }
  });

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
      "npm:top-level:pack",
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

test("runPublish fails preflight with consolidated missing tool and auth errors", () => {
  const root = makeFixtureRoot();
  const context = makeFakeContext({
    availableCommands: new Set(["cargo", "node", "npm"]),
    hasNpmAuth: false,
    hasPypiAuth: false
  });

  assert.throws(
    () =>
      runPublish(
        {
          root,
          dryRun: false,
          registry: "all",
          skipExisting: false
        },
        context
      ),
    /operator preflight failed:[\s\S]*npm publish requires existing npm auth state or NPM_TOKEN[\s\S]*PyPI publish requires TWINE_USERNAME\/TWINE_PASSWORD/
  );

  assert.deepEqual(context.labels(), []);
});

test("runPublish only requires tools for the selected registry", () => {
  const root = makeFixtureRoot();
  const cargoContext = makeFakeContext({
    availableCommands: new Set(["node", "cargo"])
  });
  const pypiContext = makeFakeContext({
    availableCommands: new Set(["node", "cargo"]),
    onRun(command) {
      if (command.label === "pypi:build-wheel") {
        writeFileSync(join(root, "wrapper-py", "dist", "linux-x64", "fresh.whl"), "wheel");
      }
      if (command.label === "pypi:build-sdist") {
        writeFileSync(join(root, "wrapper-py", "dist", "sdist", "fresh.tar.gz"), "sdist");
      }
    }
  });

  runPublish(
    {
      root,
      dryRun: true,
      registry: "cargo",
      skipExisting: false
    },
    cargoContext
  );
  runPublish(
    {
      root,
      dryRun: true,
      registry: "pypi",
      skipExisting: false
    },
    pypiContext
  );
});

test("runPublish npm skip-existing checks the configured root package name", () => {
  const root = makeFixtureRoot({
    wrapperPackageName: "@acme/my-project"
  });
  const context = makeFakeContext({
    npmVersions: new Set([
      "@acme/my-project@0.1.22",
      "@acme/my-project-darwin-arm64@0.1.22",
      "@acme/my-project-darwin-x64@0.1.22",
      "@acme/my-project-linux-x64@0.1.22",
      "@acme/my-project-windows-x64@0.1.22"
    ])
  });

  runPublish(
    {
      root,
      dryRun: false,
      registry: "npm",
      skipExisting: true
    },
    context
  );

  assert.equal(context.labels().includes("npm:wait-runtime-versions"), false);
  assert.equal(context.labels().includes("npm:install-build-deps"), false);
  assert.equal(context.labels().includes("npm:build"), false);
  assert.equal(context.logs().some((entry) => entry.includes("@acme/my-project@0.1.22")), true);
});

test("runPublish still fails npm preflight when npm is unavailable", () => {
  const root = makeFixtureRoot();
  const context = makeFakeContext({
    availableCommands: new Set(["node", "cargo"])
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
    /required executable not found on PATH: npm/
  );
});

test("runPublish prints the host-limit notice before running preflight", () => {
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

  assert.match(
    context.logs()[0],
    /local publish can only build the current host npm runtime package and current host Python wheel/
  );
});

test("runPublish clears stale PyPI artifacts before building", () => {
  const root = makeFixtureRoot();
  const staleWheel = join(root, "wrapper-py", "dist", "linux-x64", "stale.whl");
  const staleSdist = join(root, "wrapper-py", "dist", "sdist", "stale.tar.gz");
  const context = makeFakeContext({
    onRun(command) {
      if (command.label === "pypi:build-wheel") {
        assert.equal(existsSync(staleWheel), false);
        writeFileSync(join(root, "wrapper-py", "dist", "linux-x64", "fresh.whl"), "wheel");
      }
      if (command.label === "pypi:build-sdist") {
        assert.equal(existsSync(staleSdist), false);
        writeFileSync(join(root, "wrapper-py", "dist", "sdist", "fresh.tar.gz"), "sdist");
      }
    }
  });

  runPublish(
    {
      root,
      dryRun: false,
      registry: "pypi",
      skipExisting: false
    },
    context
  );

  assert.equal(existsSync(staleWheel), false);
  assert.equal(existsSync(staleSdist), false);
});

test("runPublish allows cargo skip-existing reruns without cargo auth", () => {
  const root = makeFixtureRoot();
  const context = makeFakeContext({
    cargoVersions: new Set(["ossplate@0.1.22"]),
    hasCargoAuth: false
  });

  runPublish(
    {
      root,
      dryRun: false,
      registry: "cargo",
      skipExisting: true
    },
    context
  );

  assert.deepEqual(context.labels(), [
    "tool:validate",
    "tool:sync-check",
    "release:assert",
    "js:lockfile-assert",
    "publish:assert"
  ]);
});

test("runPublish allows npm skip-existing reruns without npm auth", () => {
  const root = makeFixtureRoot();
  const context = makeFakeContext({
    npmVersions: new Set([
      "ossplate@0.1.22",
      "ossplate-darwin-arm64@0.1.22",
      "ossplate-darwin-x64@0.1.22",
      "ossplate-linux-x64@0.1.22",
      "ossplate-windows-x64@0.1.22"
    ]),
    hasNpmAuth: false
  });

  runPublish(
    {
      root,
      dryRun: false,
      registry: "npm",
      skipExisting: true
    },
    context
  );

  assert.equal(context.labels().includes("npm:top-level:publish"), false);
});

test("runPublish rejects multiple new PyPI artifacts in the same output directory", () => {
  const root = makeFixtureRoot();
  rmSync(join(root, "wrapper-py", "dist", "linux-x64"), { recursive: true, force: true });
  rmSync(join(root, "wrapper-py", "dist", "sdist"), { recursive: true, force: true });
  mkdirSync(join(root, "wrapper-py", "dist", "linux-x64"), { recursive: true });
  mkdirSync(join(root, "wrapper-py", "dist", "sdist"), { recursive: true });

  const context = makeFakeContext({
    onRun(command) {
      if (command.label === "pypi:build-wheel") {
        writeFileSync(join(root, "wrapper-py", "dist", "linux-x64", "one.whl"), "wheel");
        writeFileSync(join(root, "wrapper-py", "dist", "linux-x64", "two.whl"), "wheel");
      }
      if (command.label === "pypi:build-sdist") {
        writeFileSync(join(root, "wrapper-py", "dist", "sdist", "fresh.tar.gz"), "sdist");
      }
    }
  });

  assert.throws(
    () =>
      runPublish(
        {
          root,
          dryRun: false,
          registry: "pypi",
          skipExisting: false
        },
        context
      ),
    /multiple wheel artifacts/
  );
});

test("waitForNpmVersions reports propagation-oriented timeout diagnostics", () => {
  const logs = [];

  assert.throws(
    () =>
      waitForNpmVersions({
        version: "0.1.22",
        packages: ["ossplate-darwin-arm64", "ossplate-linux-x64"],
        attempts: 2,
        delayMs: 0,
        npmVersionExists() {
          return false;
        },
        log(message) {
          logs.push(message);
        }
      }),
    /npm runtime propagation timeout after 2 attempts \(0s total wait\):[\s\S]*verify the package names and published versions/
  );

  assert.match(logs[0], /waiting for npm runtime propagation \(1\/2\): still missing/);
});

test("buildCommandEnv only forwards registry auth to matching publish commands", () => {
  const baseEnv = {
    PATH: "/usr/bin",
    NPM_TOKEN: "npm-secret",
    CARGO_REGISTRY_TOKEN: "cargo-secret",
    TWINE_USERNAME: "__token__",
    TWINE_PASSWORD: "pypi-secret",
    KEEP_ME: "yes"
  };

  const npmPublishEnv = buildCommandEnv(baseEnv, { label: "npm:top-level:publish" });
  assert.equal(npmPublishEnv.NPM_TOKEN, "npm-secret");
  assert.equal("CARGO_REGISTRY_TOKEN" in npmPublishEnv, false);
  assert.equal("TWINE_PASSWORD" in npmPublishEnv, false);

  const cargoPublishEnv = buildCommandEnv(baseEnv, { label: "cargo:publish" });
  assert.equal(cargoPublishEnv.CARGO_REGISTRY_TOKEN, "cargo-secret");
  assert.equal("NPM_TOKEN" in cargoPublishEnv, false);
  assert.equal("TWINE_USERNAME" in cargoPublishEnv, false);

  const pypiUploadEnv = buildCommandEnv(baseEnv, { label: "pypi:upload" });
  assert.equal(pypiUploadEnv.TWINE_USERNAME, "__token__");
  assert.equal(pypiUploadEnv.TWINE_PASSWORD, "pypi-secret");
  assert.equal("NPM_TOKEN" in pypiUploadEnv, false);

  const buildEnv = buildCommandEnv(baseEnv, {
    label: "npm:build",
    env: { OSSPLATE_PY_TARGET: "linux-x64" }
  });
  assert.equal(buildEnv.KEEP_ME, "yes");
  assert.equal(buildEnv.OSSPLATE_PY_TARGET, "linux-x64");
  assert.equal("NPM_TOKEN" in buildEnv, false);
  assert.equal("CARGO_REGISTRY_TOKEN" in buildEnv, false);
  assert.equal("TWINE_PASSWORD" in buildEnv, false);
});

function makeFixtureRoot({ wrapperPackageName = "ossplate" } = {}) {
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
  writeFileSync(join(root, "scripts", "release-check.mjs"), "console.log('ok')\n");
  writeFileSync(join(root, "scripts", "assert-js-lockfile-state.mjs"), "console.log('ok')\n");
  writeFileSync(join(root, "scripts", "wait-for-npm-versions.mjs"), "console.log('ok')\n");
  writeFileSync(
    join(root, "wrapper-js", "package.json"),
    JSON.stringify(
      {
        name: wrapperPackageName,
        version: "0.1.22",
        optionalDependencies: {
          [`${wrapperPackageName}-darwin-arm64`]: "0.1.22",
          [`${wrapperPackageName}-darwin-x64`]: "0.1.22",
          [`${wrapperPackageName}-linux-x64`]: "0.1.22",
          [`${wrapperPackageName}-windows-x64`]: "0.1.22"
        }
      },
      null,
      2
    )
  );
  return root;
}

function makeFakeContext({
  failOnLabel = null,
  npmVersions = new Set(),
  cargoVersions = new Set(),
  availableCommands = new Set(["cargo", "node", "npm", "curl"]),
  hasNpmAuth = true,
  hasCargoAuth = true,
  hasPypiAuth = true,
  onRun = null
} = {}) {
  const executed = [];
  const logLines = [];
  return {
    run(command) {
      executed.push(command);
      if (onRun) {
        onRun(command);
      }
      if (command.label === failOnLabel) {
        throw new Error(`forced failure for ${command.label}`);
      }
    },
    capture(command) {
      executed.push(command);
      if (onRun) {
        onRun(command);
      }
      if (command.label === failOnLabel) {
        throw new Error(`forced failure for ${command.label}`);
      }
      return "/tmp/ossplate-test-package/ossplate-0.0.0.tgz\n";
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
    commandExists(command) {
      return availableCommands.has(command);
    },
    hasNpmAuth() {
      return hasNpmAuth;
    },
    hasCargoAuth() {
      return hasCargoAuth;
    },
    hasPypiAuth() {
      return hasPypiAuth;
    },
    platform() {
      return "linux";
    },
    arch() {
      return "x64";
    },
    log(message) {
      logLines.push(message);
    },
    labels() {
      return executed.map((command) => command.label);
    },
    commands() {
      return executed;
    },
    logs() {
      return logLines;
    }
  };
}
