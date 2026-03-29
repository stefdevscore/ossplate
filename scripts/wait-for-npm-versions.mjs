import { execFileSync } from "node:child_process";

const version = process.argv[2];
const packages = process.argv.slice(3);
const attempts = Number(process.env.OSSPLATE_NPM_WAIT_ATTEMPTS ?? "24");
const delayMs = Number(process.env.OSSPLATE_NPM_WAIT_DELAY_MS ?? "5000");

if (!version || packages.length === 0) {
  throw new Error(
    "usage: node scripts/wait-for-npm-versions.mjs <version> <package-name> [package-name...]"
  );
}

for (let attempt = 1; attempt <= attempts; attempt += 1) {
  const missing = packages.filter((packageName) => !npmVersionExists(packageName, version));
  if (missing.length === 0) {
    console.log(`npm versions available for ${version}: ${packages.join(", ")}`);
    process.exit(0);
  }
  if (attempt === attempts) {
    throw new Error(
      `npm versions did not become available after ${attempts} attempts:\n- ${missing
        .map((name) => `${name}@${version}`)
        .join("\n- ")}`
    );
  }
  console.log(
    `waiting for npm versions (${attempt}/${attempts}): ${missing
      .map((name) => `${name}@${version}`)
      .join(", ")}`
  );
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, delayMs);
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
