import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const nextVersion = process.argv[2];

if (!nextVersion) {
  throw new Error("usage: node scripts/bump-version.mjs <version>");
}

replaceInFile(joinPath("core-rs", "Cargo.toml"), /^version = ".*"$/m, `version = "${nextVersion}"`);
replaceInFile(joinPath("wrapper-py", "pyproject.toml"), /^version = ".*"$/m, `version = "${nextVersion}"`);

const packageJsonPath = joinPath("wrapper-js", "package.json");
const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf8"));
packageJson.version = nextVersion;
writeJson(packageJsonPath, packageJson);

const packageLockPath = joinPath("wrapper-js", "package-lock.json");
const packageLock = JSON.parse(readFileSync(packageLockPath, "utf8"));
packageLock.version = nextVersion;
if (packageLock.packages?.[""]) {
  packageLock.packages[""].version = nextVersion;
}
writeJson(packageLockPath, packageLock);

replaceInFile(joinPath("core-rs", "src", "main.rs"), /version = "\d+\.\d+\.\d+"/g, `version = "${nextVersion}"`);

exec("cargo", ["generate-lockfile", "--manifest-path", joinPath("core-rs", "Cargo.toml")]);
exec("npm", ["install", "--package-lock-only"], joinPath("wrapper-js"));
exec("cargo", ["run", "--quiet", "--manifest-path", joinPath("core-rs", "Cargo.toml"), "--", "sync", "--path", joinPath()]);
exec("node", [joinPath("scripts", "stage-distribution-assets.mjs")]);

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
