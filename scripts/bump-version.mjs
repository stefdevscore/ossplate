import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { getRuntimeTargets, runtimePackageFolder } from "./runtime-targets.mjs";

const nextVersion = process.argv[2];

if (!nextVersion) {
  throw new Error("usage: node scripts/bump-version.mjs <version>");
}

replaceInFile(joinPath("core-rs", "Cargo.toml"), /^version = ".*"$/m, `version = "${nextVersion}"`);
replaceInFile(joinPath("wrapper-py", "pyproject.toml"), /^version = ".*"$/m, `version = "${nextVersion}"`);

const packageJsonPath = joinPath("wrapper-js", "package.json");
const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf8"));
packageJson.version = nextVersion;
for (const packageName of Object.keys(packageJson.optionalDependencies ?? {})) {
  packageJson.optionalDependencies[packageName] = nextVersion;
}
writeJson(packageJsonPath, packageJson);

for (const packageName of getRuntimeTargets().map((entry) => runtimePackageFolder(entry.target))) {
  const runtimePackagePath = joinPath("wrapper-js", "platform-packages", packageName, "package.json");
  const runtimePackageJson = JSON.parse(readFileSync(runtimePackagePath, "utf8"));
  runtimePackageJson.version = nextVersion;
  writeJson(runtimePackagePath, runtimePackageJson);
}

const packageLockPath = joinPath("wrapper-js", "package-lock.json");
const packageLock = JSON.parse(readFileSync(packageLockPath, "utf8"));
packageLock.version = nextVersion;
if (packageLock.packages?.[""]) {
  packageLock.packages[""].version = nextVersion;
  packageLock.packages[""].optionalDependencies = {
    ...(packageJson.optionalDependencies ?? {})
  };
}
for (const packageName of Object.keys(packageLock.packages ?? {})) {
  if (packageName.startsWith("node_modules/ossplate-")) {
    delete packageLock.packages[packageName];
  }
}
for (const packageName of Object.keys(packageJson.optionalDependencies ?? {})) {
  packageLock.packages[`node_modules/${packageName}`] = {
    optional: true
  };
}
writeJson(packageLockPath, packageLock);

replaceInFile(joinPath("core-rs", "src", "main.rs"), /version = "\d+\.\d+\.\d+"/g, `version = "${nextVersion}"`);

exec("cargo", ["generate-lockfile", "--manifest-path", joinPath("core-rs", "Cargo.toml")]);
exec("node", [joinPath("scripts", "stage-distribution-assets.mjs"), "embedded-template"]);
exec("cargo", ["run", "--quiet", "--manifest-path", joinPath("core-rs", "Cargo.toml"), "--", "sync", "--path", joinPath()]);

function replaceInFile(filePath, pattern, replacement) {
  const content = readFileSync(filePath, "utf8");
  const next = content.replace(pattern, replacement);
  writeFileSync(filePath, next);
}

function writeJson(filePath, value) {
  writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

function exec(command, args, cwd = joinPath()) {
  execFileSync(command, args, {
    cwd,
    stdio: "inherit"
  });
}

function joinPath(...parts) {
  return join(process.cwd(), ...parts);
}
