import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const packageJson = readJson("wrapper-js/package.json");
const packageLock = readJson("wrapper-js/package-lock.json");

main();

function main() {
  const expectedVersion = packageJson.version;
  const expectedOptionalDependencies = packageJson.optionalDependencies ?? {};
  const rootPackage = packageLock.packages?.[""];

  if (packageLock.version !== expectedVersion) {
    fail(
      `wrapper-js/package-lock.json version is ${packageLock.version}; expected ${expectedVersion}`
    );
  }

  if (!rootPackage) {
    fail('wrapper-js/package-lock.json is missing packages[""]');
  }

  if (rootPackage.version !== expectedVersion) {
    fail(
      `wrapper-js/package-lock.json packages[\"\"].version is ${rootPackage.version}; expected ${expectedVersion}`
    );
  }

  assertDeepEqual(
    rootPackage.optionalDependencies ?? {},
    expectedOptionalDependencies,
    "wrapper-js/package-lock.json root optionalDependencies must match wrapper-js/package.json"
  );

  const expectedPackages = new Set(Object.keys(expectedOptionalDependencies));
  const lockEntries = Object.entries(packageLock.packages ?? {}).filter(([entryPath]) =>
    entryPath.startsWith("node_modules/ossplate-")
  );

  for (const [entryPath, entry] of lockEntries) {
    const packageName = entryPath.slice("node_modules/".length);
    if (!expectedPackages.has(packageName)) {
      fail(`unexpected runtime package entry in lockfile: ${entryPath}`);
    }
    if (entry.optional !== true) {
      fail(`lockfile runtime package ${entryPath} must be marked optional`);
    }
    const expectedEntryVersion = expectedOptionalDependencies[packageName];
    if (entry.version !== undefined && entry.version !== expectedEntryVersion) {
      fail(
        `lockfile runtime package ${entryPath} has version ${entry.version}; expected ${expectedEntryVersion}`
      );
    }
  }

  for (const packageName of expectedPackages) {
    const entryPath = `node_modules/${packageName}`;
    if (!packageLock.packages?.[entryPath]) {
      fail(`missing runtime package entry in lockfile: ${entryPath}`);
    }
  }

  console.log("js lockfile state ok");
}

function readJson(relativePath) {
  return JSON.parse(readFileSync(join(repoRoot, relativePath), "utf8"));
}

function assertDeepEqual(actual, expected, message) {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    fail(`${message}\nactual=${JSON.stringify(actual)}\nexpected=${JSON.stringify(expected)}`);
  }
}

function fail(message) {
  throw new Error(message);
}
