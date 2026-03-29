import test from "node:test";
import assert from "node:assert/strict";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";

import {
  assertNpmVersionState,
  assertScaffoldMirrorsState,
  getExpectedOptionalDependencies,
  readTomlSectionValue
} from "./release-state.mjs";

test("expected optional dependencies are derived from the runtime contract once", () => {
  const rootPackage = {
    name: "ossplate",
    version: "1.2.3"
  };

  assert.deepEqual(getExpectedOptionalDependencies(rootPackage), {
    "ossplate-darwin-arm64": "1.2.3",
    "ossplate-darwin-x64": "1.2.3",
    "ossplate-linux-x64": "1.2.3",
    "ossplate-windows-x64": "1.2.3"
  });
});

test("publish readiness allows missing runtime versions until the top-level package exists", () => {
  const rootPackage = {
    name: "ossplate",
    version: "1.2.3",
    optionalDependencies: {
      "ossplate-darwin-arm64": "1.2.3",
      "ossplate-darwin-x64": "1.2.3"
    }
  };

  assert.doesNotThrow(() =>
    assertNpmVersionState({
      mode: "publish",
      version: "1.2.3",
      rootPackage,
      npmVersionExists: () => false
    })
  );
});

test("publish readiness fails when the top-level package exists before all runtime packages", () => {
  const rootPackage = {
    name: "ossplate",
    version: "1.2.3",
    optionalDependencies: {
      "ossplate-darwin-arm64": "1.2.3",
      "ossplate-darwin-x64": "1.2.3"
    }
  };

  assert.throws(
    () =>
      assertNpmVersionState({
        mode: "publish",
        version: "1.2.3",
        rootPackage,
        npmVersionExists: (packageName) => packageName === "ossplate"
      }),
    /top-level package ossplate@1\.2\.3 exists without all runtime packages/
  );
});

test("release readiness fails when any npm package version is already published", () => {
  const rootPackage = {
    name: "ossplate",
    version: "1.2.3",
    optionalDependencies: {
      "ossplate-darwin-arm64": "1.2.3",
      "ossplate-darwin-x64": "1.2.3"
    }
  };

  assert.throws(
    () =>
      assertNpmVersionState({
        mode: "release",
        version: "1.2.3",
        rootPackage,
        npmVersionExists: (packageName) => packageName === "ossplate-darwin-arm64"
      }),
    /release preflight requires a clean npm version state/
  );
});

test("scaffold mirror assertion uses the payload contract and fails on drift", () => {
  const root = mkTempTree();
  const payload = { requiredPaths: ["README.md"] };
  const source = path.join(root, "README.md");
  const jsMirror = path.join(root, "wrapper-js", "scaffold", "README.md");
  const pyMirror = path.join(root, "wrapper-py", "src", "ossplate", "scaffold", "README.md");

  mkdirSync(path.dirname(source), { recursive: true });
  mkdirSync(path.dirname(jsMirror), { recursive: true });
  mkdirSync(path.dirname(pyMirror), { recursive: true });

  writeFileSync(source, "root\n");
  writeFileSync(jsMirror, "root\n");
  writeFileSync(pyMirror, "root\n");

  assert.doesNotThrow(() =>
    assertScaffoldMirrorsState(payload, {
      root,
      scaffoldRoots: [path.join(root, "wrapper-js", "scaffold"), path.join(root, "wrapper-py", "src", "ossplate", "scaffold")]
    })
  );

  writeFileSync(pyMirror, "drift\n");
  assert.throws(
    () =>
      assertScaffoldMirrorsState(payload, {
        root,
        scaffoldRoots: [path.join(root, "wrapper-js", "scaffold"), path.join(root, "wrapper-py", "src", "ossplate", "scaffold")]
      }),
    /scaffold snapshot drift detected for README\.md/
  );

  rmSync(root, { recursive: true, force: true });
});

test("Cargo version reading is scoped to the package section", () => {
  const cargoToml = `
[dependencies.clap]
version = "4.5"

[package]
name = "agentcode"
version = "0.2.3"
`;

  assert.equal(
    readTomlSectionValue(cargoToml, "package", "version", "core-rs/Cargo.toml"),
    "0.2.3"
  );
});

function mkTempTree() {
  return path.join(os.tmpdir(), `ossplate-release-state-${process.pid}-${Date.now()}-${Math.random().toString(16).slice(2)}`);
}
