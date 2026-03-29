import test from "node:test";
import assert from "node:assert/strict";

import {
  bumpVersion,
  classifyBump,
  isAutomatedReleaseCommit
} from "./release-plan.mjs";

test("filters automated release commits from planning", () => {
  assert.equal(isAutomatedReleaseCommit("Release ossplate 0.1.22 [skip ci]"), true);
  assert.equal(
    isAutomatedReleaseCommit("Sync JS lockfile after ossplate 0.1.22 release [skip ci]"),
    true
  );
  assert.equal(isAutomatedReleaseCommit("fix: repair npm publish workflow"), false);
});

test("classifies normal feature work even with automated release commits around it", () => {
  const commits = [
    "Release ossplate 0.1.22 [skip ci]",
    "Sync JS lockfile after ossplate 0.1.22 release [skip ci]",
    "feat: add better sync diagnostics"
  ].filter((entry) => !isAutomatedReleaseCommit(entry));

  assert.deepEqual(commits, ["feat: add better sync diagnostics"]);
  assert.equal(classifyBump(commits), "minor");
  assert.equal(bumpVersion("0.1.22", "minor"), "0.2.0");
});
