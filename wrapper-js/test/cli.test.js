import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const wrapperRoot = path.resolve(__dirname, "..");
const repoRoot = path.resolve(wrapperRoot, "..");
const distModule = path.join(wrapperRoot, "dist", "index.js");
const stubBinary = path.join(__dirname, "fixtures", "ossplate-stub.sh");
const scaffoldManifest = JSON.parse(
  fs.readFileSync(path.join(repoRoot, "scaffold-manifest.json"), "utf8")
);

async function loadModule() {
  return import(pathToFileURL(distModule).href);
}

const supportedTargets = [
  ["darwin", "arm64", "darwin-arm64", "ossplate"],
  ["darwin", "x64", "darwin-x64", "ossplate"],
  ["linux", "x64", "linux-x64", "ossplate"],
  ["win32", "x64", "win32-x64", "ossplate.exe"]
];

test("env override takes precedence for wrapper execution", () => {
  const output = execFileSync("node", ["bin/ossplate.js", "version"], {
    cwd: wrapperRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      OSSPLATE_BINARY: stubBinary
    }
  }).trim();

  assert.equal(output, '{"tool":"stub-tool","version":"9.9.9"}');
});

test("resolveOssplateBinary honors env override", async () => {
  const { resolveOssplateBinary } = await loadModule();
  process.env.OSSPLATE_BINARY = stubBinary;
  try {
    assert.equal(resolveOssplateBinary(), stubBinary);
  } finally {
    delete process.env.OSSPLATE_BINARY;
  }
});

test("resolveOssplateBinary rejects unsupported platforms", async () => {
  const { resolveOssplateBinary } = await loadModule();
  assert.throws(
    () => resolveOssplateBinary({ platform: "linux", arch: "arm64", baseDir: wrapperRoot }),
    /Unsupported platform\/arch: linux\/arm64/
  );
});

test("resolveOssplateBinary resolves every declared packaged target", async () => {
  const { resolveOssplateBinary } = await loadModule();
  for (const [platform, arch, target, executable] of supportedTargets) {
    assert.equal(
      resolveOssplateBinary({ platform, arch, baseDir: wrapperRoot }),
      path.join(wrapperRoot, "bin", target, executable)
    );
  }
});

test("js wrapper matches the rust contract via env override", () => {
  execFileSync("cargo", ["build"], {
    cwd: path.join(repoRoot, "core-rs"),
    stdio: "ignore"
  });
  const coreBinary = path.join(repoRoot, "core-rs", "target", "debug", "ossplate");
  for (const args of [
    ["version"],
    ["validate", "--path", repoRoot, "--json"],
    ["sync", "--path", repoRoot, "--check"]
  ]) {
    const direct = execFileSync(coreBinary, args, {
      cwd: repoRoot,
      encoding: "utf8"
    }).trim();
    const wrapped = execFileSync("node", ["bin/ossplate.js", ...args], {
      cwd: wrapperRoot,
      encoding: "utf8",
      env: {
        ...process.env,
        OSSPLATE_BINARY: coreBinary
      }
    }).trim();
    assert.equal(wrapped, direct);
  }
});

test("packaged js wrapper can create from scaffold payload", () => {
  execFileSync("cargo", ["build"], {
    cwd: path.join(repoRoot, "core-rs"),
    stdio: "ignore"
  });
  execFileSync("node", [path.join(repoRoot, "scripts", "stage-distribution-assets.mjs")], {
    cwd: repoRoot,
    stdio: "ignore"
  });
  execFileSync("npm", ["pack"], {
    cwd: wrapperRoot,
    stdio: "ignore"
  });
  const tarball = path.join(wrapperRoot, "ossplate-0.1.0.tgz");
  const unpackDir = path.join(wrapperRoot, ".tmp-pack");
  const targetDir = path.join(wrapperRoot, ".tmp-created");
  execFileSync("rm", ["-rf", unpackDir, targetDir], { cwd: wrapperRoot });
  execFileSync("mkdir", ["-p", unpackDir], { cwd: wrapperRoot });
  execFileSync("tar", ["-xzf", tarball, "-C", unpackDir], { cwd: wrapperRoot });
  const packagedFiles = listFiles(path.join(unpackDir, "package"));
  for (const relativePath of scaffoldManifest.requiredPaths) {
    assert.ok(
      packagedFiles.includes(path.join("scaffold", relativePath)),
      `expected packaged scaffold file ${relativePath}`
    );
  }
  for (const excludedPrefix of scaffoldManifest.excludedPrefixes) {
    assert.ok(
      !packagedFiles.some((file) => file.startsWith(path.join("scaffold", excludedPrefix))),
      `unexpected packaged scaffold file under ${excludedPrefix}`
    );
  }
  const packagedTool = path.join(unpackDir, "package", "bin", "ossplate.js");
  const directVersion = execFileSync(
    path.join(repoRoot, "core-rs", "target", "debug", "ossplate"),
    ["version"],
    {
      cwd: repoRoot,
      encoding: "utf8"
    }
  ).trim();
  const packagedVersion = execFileSync("node", [packagedTool, "version"], {
    cwd: wrapperRoot,
    encoding: "utf8"
  }).trim();
  assert.equal(packagedVersion, directVersion);
  execFileSync("node", [packagedTool, "create", targetDir], {
    cwd: wrapperRoot,
    stdio: "ignore"
  });
  const output = execFileSync("node", [packagedTool, "validate", "--path", targetDir, "--json"], {
    cwd: wrapperRoot,
    encoding: "utf8"
  }).trim();
  assert.equal(output, '{"ok":true,"issues":[]}');
  execFileSync("rm", ["-rf", unpackDir, targetDir, tarball], { cwd: wrapperRoot });
});

function listFiles(rootDir) {
  const results = [];

  walk(rootDir, "");
  return results.sort();

  function walk(currentDir, prefix) {
    for (const entry of fs.readdirSync(currentDir, { withFileTypes: true })) {
      const relativePath = path.join(prefix, entry.name);
      const fullPath = path.join(currentDir, entry.name);
      if (entry.isDirectory()) {
        walk(fullPath, relativePath);
      } else {
        results.push(relativePath);
      }
    }
  }
}
