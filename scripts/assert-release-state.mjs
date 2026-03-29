import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const scaffoldManifest = readJson("scaffold-manifest.json");
const rootPackage = readJson("wrapper-js/package.json");
const runtimeTargets = [
  "darwin-arm64",
  "darwin-x64",
  "linux-x64",
  "win32-x64"
];
const runtimePackageFolders = Object.fromEntries(
  runtimeTargets.map((target) => [target, `ossplate-${target}`])
);

main();

function main() {
  const versions = {
    rust: readCargoVersion(),
    js: rootPackage.version,
    python: readPyprojectVersion()
  };

  assertAllEqual(Object.entries(versions), "package versions must stay aligned");
  assertOptionalDependencies();
  assertRuntimePackages();
  assertNoTrackedGeneratedBinaries();
  assertScaffoldSnapshots();
  assertTopLevelPackShape();
  console.log("release state ok");
}

function assertOptionalDependencies() {
  const expected = Object.fromEntries(
    runtimeTargets.map((target) => [runtimePackageName(target), rootPackage.version])
  );
  const actual = rootPackage.optionalDependencies ?? {};
  assertDeepEqual(
    actual,
    expected,
    "wrapper-js/package.json optionalDependencies must match the runtime package contract"
  );
}

function assertRuntimePackages() {
  for (const target of runtimeTargets) {
    const folder = runtimePackageFolders[target];
    const packageJson = readJson(join("wrapper-js", "platform-packages", folder, "package.json"));
    if (packageJson.name !== runtimePackageName(target)) {
      fail(
        `runtime package ${folder} has name ${packageJson.name}; expected ${runtimePackageName(target)}`
      );
    }
    if (packageJson.version !== rootPackage.version) {
      fail(
        `runtime package ${packageJson.name} has version ${packageJson.version}; expected ${rootPackage.version}`
      );
    }
  }
}

function assertNoTrackedGeneratedBinaries() {
  const tracked = execGit(["ls-files"])
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean)
    .filter(
      (line) =>
        /^wrapper-js\/platform-packages\/[^/]+\/bin\//.test(line) ||
        /^wrapper-py\/src\/ossplate\/bin\/[^/]+\/ossplate(?:\.exe)?$/.test(line)
    );
  if (tracked.length > 0) {
    fail(
      `generated runtime binaries must not be tracked:\n${tracked
        .map((entry) => `- ${entry}`)
        .join("\n")}`
    );
  }
}

function assertScaffoldSnapshots() {
  const scaffoldRoots = [
    join(repoRoot, "wrapper-js", "scaffold"),
    join(repoRoot, "wrapper-py", "src", "ossplate", "scaffold")
  ];
  for (const relativePath of scaffoldManifest.requiredPaths) {
    const source = join(repoRoot, relativePath);
    if (!existsSync(source)) {
      fail(`missing required scaffold source path ${relativePath}`);
    }
    const sourceContent = readFileSync(source);
    for (const scaffoldRoot of scaffoldRoots) {
      const snapshotPath = join(scaffoldRoot, relativePath);
      if (!existsSync(snapshotPath)) {
        fail(`missing scaffold snapshot ${relative(scaffoldRoot, snapshotPath)}`);
      }
      const snapshotContent = readFileSync(snapshotPath);
      if (!sourceContent.equals(snapshotContent)) {
        fail(`scaffold snapshot drift detected for ${relativePath}`);
      }
    }
  }
}

function assertTopLevelPackShape() {
  const output = execFileSync("npm", ["pack", "--dry-run", "--json"], {
    cwd: join(repoRoot, "wrapper-js"),
    encoding: "utf8"
  });
  const parsed = JSON.parse(output);
  const files = parsed[0]?.files?.map((entry) => entry.path) ?? [];
  for (const file of files) {
    if (/^bin\/(darwin|linux|win32)-/.test(file)) {
      fail(`top-level npm package still contains bundled runtime binary path ${file}`);
    }
    if (/^scaffold\/wrapper-js\/bin\/(darwin|linux|win32)-/.test(file)) {
      fail(`scaffold still contains nested JS runtime binary path ${file}`);
    }
    if (file.startsWith("scaffold/wrapper-py/src/ossplate/bin/")) {
      fail(`scaffold still contains nested Python runtime binary path ${file}`);
    }
  }
}

function readCargoVersion() {
  const cargoToml = readText("core-rs/Cargo.toml");
  const match = cargoToml.match(/^version = "([^"]+)"$/m);
  if (!match) {
    fail("failed to read version from core-rs/Cargo.toml");
  }
  return match[1];
}

function readPyprojectVersion() {
  const pyproject = readText("wrapper-py/pyproject.toml");
  const match = pyproject.match(/^version = "([^"]+)"$/m);
  if (!match) {
    fail("failed to read version from wrapper-py/pyproject.toml");
  }
  return match[1];
}

function runtimePackageName(target) {
  return `${rootPackage.name}-${target}`;
}

function readText(relativePath) {
  return readFileSync(join(repoRoot, relativePath), "utf8");
}

function readJson(relativePath) {
  return JSON.parse(readText(relativePath));
}

function execGit(args) {
  return execFileSync("git", args, {
    cwd: repoRoot,
    encoding: "utf8"
  }).trim();
}

function assertAllEqual(entries, message) {
  const values = new Set(entries.map(([, value]) => value));
  if (values.size !== 1) {
    fail(
      `${message}: ${entries
        .map(([name, value]) => `${name}=${value}`)
        .join(", ")}`
    );
  }
}

function assertDeepEqual(actual, expected, message) {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    fail(`${message}\nactual=${JSON.stringify(actual)}\nexpected=${JSON.stringify(expected)}`);
  }
}

function fail(message) {
  throw new Error(message);
}
