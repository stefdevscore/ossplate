import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";

import {
  assertProjectMetadataReady,
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
  const payload = { requiredPaths: ["README.md"], templateOnlyPaths: ["docs/live-e2e.md"] };
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

test("generated scaffold assets reject embedded template-only docs", () => {
  const root = mkTempTree();
  const payload = { requiredPaths: ["README.md"], templateOnlyPaths: ["docs/live-e2e.md"] };
  const source = path.join(root, "README.md");

  mkdirSync(path.dirname(source), { recursive: true });
  writeFileSync(source, "root\n");

  assert.throws(
    () =>
      assertGeneratedScaffoldAssets(payload, {
        root,
        pythonPackageSrcDir: "src/ossplate",
        stageScaffoldPackage(_repoRoot, scaffoldRoot) {
          const scaffoldFile = path.join(scaffoldRoot, "README.md");
          const embeddedFile = path.join(scaffoldRoot, "core-rs", "embedded-template-root", "ossplate.toml");
          const leakedDoc = path.join(scaffoldRoot, "core-rs", "embedded-template-root", "docs", "live-e2e.md");
          mkdirSync(path.dirname(scaffoldFile), { recursive: true });
          mkdirSync(path.dirname(embeddedFile), { recursive: true });
          mkdirSync(path.dirname(leakedDoc), { recursive: true });
          writeFileSync(scaffoldFile, "root\n");
          writeFileSync(embeddedFile, "ok\n");
          writeFileSync(leakedDoc, "leak\n");
        }
      }),
    /generated embedded template must not reintroduce template-only path docs\/live-e2e\.md/
  );

  rmSync(root, { recursive: true, force: true });
});

test("stage-embedded-template requires matching template-only contracts", () => {
  const root = mkTempTree();
  const scriptsDir = path.join(root, "scripts");
  mkdirSync(scriptsDir, { recursive: true });
  writeFileSync(
    path.join(root, "scaffold-payload.json"),
    JSON.stringify({ requiredPaths: ["README.md"], templateOnlyPaths: ["docs/live-e2e.md"] }, null, 2)
  );
  writeFileSync(
    path.join(root, "source-checkout.json"),
    JSON.stringify({ requiredPaths: ["README.md"], templateOnlyPaths: [] }, null, 2)
  );
  writeFileSync(path.join(root, "ossplate.toml"), "[template]\nis_canonical = true\n");
  writeFileSync(path.join(root, "README.md"), "root\n");
  writeFileSync(
    path.join(scriptsDir, "stage-embedded-template.mjs"),
    readFileSync(path.join(path.dirname(new URL(import.meta.url).pathname), "stage-embedded-template.mjs"), "utf8")
  );

  assert.throws(
    () =>
      execFileSync("node", [path.join(root, "scripts", "stage-embedded-template.mjs"), path.join(root, "out")], {
        cwd: root,
        encoding: "utf8"
      }),
    /templateOnlyPaths must match between scaffold-payload\.json and source-checkout\.json/
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

test("publish readiness rejects generated placeholder project metadata", () => {
  assert.throws(
    () =>
      assertProjectMetadataReady(
        {
          projectDescription:
            "Ship the `ossblade` CLI through Cargo, npm, and PyPI. Replace this description before release.",
          projectRepository: "https://example.com/replace-with-your-repository",
          authorName: "TODO: set author name",
          authorEmail: "you@example.com",
          command: "ossblade"
        },
        { name: "ossblade" }
      ),
    /publish readiness requires real project metadata/
  );
});

test("publish readiness accepts real project metadata", () => {
  assert.doesNotThrow(() =>
    assertProjectMetadataReady(
      {
        projectDescription: "Ship the `ossblade` CLI across Cargo, npm, and PyPI.",
        projectRepository: "https://github.com/acme/ossblade",
        authorName: "Acme",
        authorEmail: "oss@acme.dev",
        command: "ossblade"
      },
      { name: "ossblade" }
    )
  );
});

function mkTempTree() {
  return path.join(os.tmpdir(), `ossplate-release-state-${process.pid}-${Date.now()}-${Math.random().toString(16).slice(2)}`);
}
