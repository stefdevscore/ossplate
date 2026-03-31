import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const version = process.argv[2];
const packages = process.argv.slice(3);
const attempts = Number(process.env.OSSPLATE_NPM_WAIT_ATTEMPTS ?? "24");
const delayMs = Number(process.env.OSSPLATE_NPM_WAIT_DELAY_MS ?? "5000");

if (isMainModule()) {
  if (!version || packages.length === 0) {
    throw new Error(
      "usage: node scripts/wait-for-npm-versions.mjs <version> <package-name> [package-name...]"
    );
  }
  waitForNpmVersions({
    version,
    packages,
    attempts,
    delayMs,
    npmVersionExists,
    log: console.log
  });
}

export function waitForNpmVersions({
  version,
  packages,
  attempts,
  delayMs,
  npmVersionExists,
  log
}) {
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    const missing = packages.filter((packageName) => !npmVersionExists(packageName, version));
    if (missing.length === 0) {
      log(`npm versions available for ${version}: ${packages.join(", ")}`);
      return;
    }
    if (attempt === attempts) {
      const totalWaitSeconds = Number(((attempts - 1) * delayMs / 1000).toFixed(1));
      throw new Error(
        `npm runtime propagation timeout after ${attempts} attempts (${totalWaitSeconds}s total wait):\n- ${missing
          .map((name) => `${name}@${version}`)
          .join("\n- ")}\nverify the package names and published versions, then rerun once npm shows those runtimes.`
      );
    }
    log(
      `waiting for npm runtime propagation (${attempt}/${attempts}): still missing ${missing
        .map((name) => `${name}@${version}`)
        .join(", ")}`
    );
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, delayMs);
  }
}

function npmVersionExists(packageName, packageVersion) {
  try {
    execFileSync("npm", ["view", `${packageName}@${packageVersion}`, "version"], {
      stdio: "ignore"
    });
    return true;
  } catch {
    return false;
  }
}

function isMainModule() {
  return process.argv[1] === fileURLToPath(import.meta.url);
}
