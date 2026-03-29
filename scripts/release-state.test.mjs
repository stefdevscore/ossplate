import test from "node:test";
import assert from "node:assert/strict";

import {
  assertNpmVersionState,
  getExpectedOptionalDependencies
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
