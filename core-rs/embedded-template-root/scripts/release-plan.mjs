import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = new URL("..", import.meta.url);
const AUTOMATED_RELEASE_SUBJECT_PATTERNS = [
  /^Release ossplate /,
  /^Sync JS lockfile after ossplate .* release \[skip ci\]$/
];

if (isMainModule()) {
  const currentVersion = readCurrentVersion();
  const latestTag = readLatestTag();
  const commits = readCommitsSince(latestTag);
  const bump = classifyBump(commits);

  const result = {
    currentVersion,
    latestTag,
    commitCount: commits.length,
    bump,
    shouldRelease: commits.length > 0,
    nextVersion: commits.length > 0 ? bumpVersion(currentVersion, bump) : currentVersion
  };

  console.log(JSON.stringify(result));
}

function readCurrentVersion() {
  const cargoToml = readFileSync(joinPath("core-rs", "Cargo.toml"), "utf8");
  const match = cargoToml.match(/^version = "([^"]+)"$/m);
  if (!match) {
    throw new Error("failed to read current version from core-rs/Cargo.toml");
  }
  return match[1];
}

function readLatestTag() {
  const output = execGit(["tag", "--list", "v*", "--sort=-version:refname"]).trim();
  if (!output) {
    return null;
  }
  return output.split("\n")[0];
}

export function readCommitsSince(tag) {
  const range = tag ? `${tag}..HEAD` : "HEAD";
  const output = execGit(["log", "--format=%s%n%b%x00", range]);
  return output
    .split("\0")
    .map((entry) => entry.trim())
    .filter(Boolean)
    .filter((entry) => !isAutomatedReleaseCommit(entry));
}

export function classifyBump(commits) {
  let bump = "patch";
  for (const commit of commits) {
    const lower = commit.toLowerCase();
    if (
      lower.includes("[major]") ||
      lower.includes("breaking change") ||
      /^[a-z]+(\(.+\))?!:/.test(commit.split("\n")[0])
    ) {
      return "major";
    }
    if (lower.includes("[minor]") || /^feat(\(.+\))?:/.test(commit.split("\n")[0])) {
      bump = "minor";
    }
    if (lower.includes("[patch]")) {
      bump = "patch";
    }
  }
  return bump;
}

export function bumpVersion(version, bump) {
  const [major, minor, patch] = version.split(".").map(Number);
  if ([major, minor, patch].some(Number.isNaN)) {
    throw new Error(`invalid version: ${version}`);
  }
  if (bump === "major") {
    return `${major + 1}.0.0`;
  }
  if (bump === "minor") {
    return `${major}.${minor + 1}.0`;
  }
  return `${major}.${minor}.${patch + 1}`;
}

function execGit(args) {
  return execFileSync("git", args, {
    cwd: joinPath(),
    encoding: "utf8"
  });
}

function joinPath(...parts) {
  return join(new URL(repoRoot).pathname, ...parts);
}

export function isAutomatedReleaseCommit(entry) {
  const subject = entry.split("\n")[0] ?? "";
  return AUTOMATED_RELEASE_SUBJECT_PATTERNS.some((pattern) => pattern.test(subject));
}

function isMainModule() {
  return process.argv[1] === fileURLToPath(import.meta.url);
}
