import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const mode = process.argv[2] ?? "publish";
const explicitVersion = process.argv[3];
const rootPackage = readJson("wrapper-js/package.json");

const version = explicitVersion ?? rootPackage.version;
const runtimePackages = Object.keys(rootPackage.optionalDependencies ?? {});
const allNpmPackages = [rootPackage.name, ...runtimePackages];

main();

function main() {
  assertRuntimePackageNames();
  assertNpmVersionState();
  console.log(`publish readiness ok (${mode}, ${version})`);
}

function assertRuntimePackageNames() {
  for (const packageName of runtimePackages) {
    if (packageName.startsWith("@")) {
      fail(
        `runtime package ${packageName} is scoped; current release policy requires unscoped publishable runtime package names`
      );
    }
    if (!/^ossplate-(darwin-arm64|darwin-x64|linux-x64|win32-x64)$/.test(packageName)) {
      fail(`runtime package ${packageName} does not match the supported npm runtime package contract`);
    }
  }
}

function assertNpmVersionState() {
  const states = new Map(allNpmPackages.map((name) => [name, npmVersionExists(name, version)]));
  const runtimeStates = runtimePackages.map((name) => states.get(name));
  const topLevelExists = states.get(rootPackage.name);

  if (mode === "release") {
    if ([...states.values()].some(Boolean)) {
      const published = [...states.entries()]
        .filter(([, exists]) => exists)
        .map(([name]) => `${name}@${version}`);
      fail(
        `release preflight requires a clean npm version state; already published package versions detected:\n- ${published.join("\n- ")}`
      );
    }
    return;
  }

  const allRuntimeExist = runtimeStates.every(Boolean);
  if (topLevelExists && !allRuntimeExist) {
    fail(`top-level package ${rootPackage.name}@${version} exists without all runtime packages`);
  }
}

function npmVersionExists(packageName, packageVersion) {
  try {
    execFileSync(npmCommand(), ["view", `${packageName}@${packageVersion}`, "version"], {
      cwd: repoRoot,
      stdio: "ignore"
    });
    return true;
  } catch {
    return false;
  }
}

function readJson(relativePath) {
  return JSON.parse(readFileSync(join(repoRoot, relativePath), "utf8"));
}

function npmCommand() {
  return process.platform === "win32" ? "npm.cmd" : "npm";
}

function fail(message) {
  throw new Error(message);
}
