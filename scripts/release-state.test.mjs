import test from "node:test";
import assert from "node:assert/strict";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import {
  assertNpmVersionState,
  assertRuntimePackageNames,
  assertGeneratedScaffoldAssets,
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

test("scoped root npm packages derive accepted scoped runtime package names", () => {
  const rootPackage = {
    name: "@acme/my-project",
    version: "1.2.3",
    optionalDependencies: {
      "@acme/my-project-darwin-arm64": "1.2.3",
      "@acme/my-project-darwin-x64": "1.2.3",
      "@acme/my-project-linux-x64": "1.2.3",
      "@acme/my-project-windows-x64": "1.2.3"
    }
  };

  assert.deepEqual(getExpectedOptionalDependencies(rootPackage), rootPackage.optionalDependencies);
  assert.doesNotThrow(() => assertRuntimePackageNames(rootPackage));
  assert.doesNotThrow(() =>
    assertNpmVersionState({
      mode: "publish",
      version: "1.2.3",
      rootPackage,
      npmVersionExists: () => false
    })
  );
});

test("generated scaffold assets are validated from canon", () => {
  const root = mkTempTree();
  const payload = { requiredPaths: ["README.md"] };
  const source = path.join(root, "README.md");

  mkdirSync(path.dirname(source), { recursive: true });
  writeFileSync(source, "root\n");

  assert.doesNotThrow(() =>
    assertGeneratedScaffoldAssets(payload, {
      root,
      pythonPackageSrcDir: "src/ossplate",
      stageScaffoldPackage(_repoRoot, scaffoldRoot) {
        const scaffoldFile = path.join(scaffoldRoot, "README.md");
        const embeddedFile = path.join(scaffoldRoot, "core-rs", "embedded-template-root", "ossplate.toml");
        mkdirSync(path.dirname(scaffoldFile), { recursive: true });
        mkdirSync(path.dirname(embeddedFile), { recursive: true });
        writeFileSync(scaffoldFile, "root\n");
        writeFileSync(embeddedFile, "ok\n");
      }
    })
  );

  assert.throws(
    () =>
      assertGeneratedScaffoldAssets(payload, {
        root,
        pythonPackageSrcDir: "src/ossplate",
        stageScaffoldPackage(_repoRoot, scaffoldRoot) {
          const scaffoldFile = path.join(scaffoldRoot, "README.md");
          const embeddedFile = path.join(scaffoldRoot, "core-rs", "embedded-template-root", "ossplate.toml");
          mkdirSync(path.dirname(scaffoldFile), { recursive: true });
          mkdirSync(path.dirname(embeddedFile), { recursive: true });
          writeFileSync(scaffoldFile, "drift\n");
          writeFileSync(embeddedFile, "ok\n");
        }
      }),
    /scaffold snapshot drift detected for README\.md/
  );

  rmSync(root, { recursive: true, force: true });
});

test("scaffold-package stages embedded template from canon without rewriting the repo snapshot", () => {
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const embeddedRoot = path.join(repoRoot, "core-rs", "embedded-template-root");
  const embeddedConfig = path.join(embeddedRoot, "ossplate.toml");
  const originalEmbeddedConfig = readFileSync(embeddedConfig, "utf8");
  const canonicalConfig = readFileSync(path.join(repoRoot, "ossplate.toml"), "utf8");
  const destinationRoot = mkTempTree();

  try {
    writeFileSync(embeddedConfig, "poisoned = true\n");
    execFileSync(
      "node",
      [path.join(repoRoot, "scripts", "stage-distribution-assets.mjs"), "scaffold-package", destinationRoot],
      {
        cwd: repoRoot,
        stdio: "ignore"
      }
    );

    assert.equal(readFileSync(embeddedConfig, "utf8"), "poisoned = true\n");
    assert.equal(
      readFileSync(path.join(destinationRoot, "core-rs", "embedded-template-root", "ossplate.toml"), "utf8"),
      canonicalConfig
    );
  } finally {
    writeFileSync(embeddedConfig, originalEmbeddedConfig);
    rmSync(destinationRoot, { recursive: true, force: true });
  }
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
