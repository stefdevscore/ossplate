import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
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
const packageJson = JSON.parse(
  fs.readFileSync(path.join(wrapperRoot, "package.json"), "utf8")
);

const supportedTargets = [
  ["darwin", "arm64", "darwin-arm64", "ossplate", "@stefdevscore/ossplate-darwin-arm64", "ossplate-darwin-arm64"],
  ["darwin", "x64", "darwin-x64", "ossplate", "@stefdevscore/ossplate-darwin-x64", "ossplate-darwin-x64"],
  ["linux", "x64", "linux-x64", "ossplate", "@stefdevscore/ossplate-linux-x64", "ossplate-linux-x64"],
  ["win32", "x64", "win32-x64", "ossplate.exe", "@stefdevscore/ossplate-win32-x64", "ossplate-win32-x64"]
];

async function loadModule() {
  return import(pathToFileURL(distModule).href);
}

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

test("resolveOssplateBinary resolves every declared runtime package target", async () => {
  const { resolveOssplateBinary } = await loadModule();
  const packagesBaseDir = fs.mkdtempSync(path.join(os.tmpdir(), "ossplate-js-runtime-resolve-"));
  try {
    for (const [_platform, _arch, _target, executable, runtimePackage] of supportedTargets) {
      const runtimeBinDir = path.join(packagesBaseDir, runtimePackage, "bin");
      fs.mkdirSync(runtimeBinDir, { recursive: true });
      const runtimeBinary = path.join(runtimeBinDir, executable);
      fs.writeFileSync(runtimeBinary, "#!/bin/sh\nexit 0\n");
      if (process.platform !== "win32") {
        fs.chmodSync(runtimeBinary, 0o755);
      }
    }

    for (const [platform, arch, _target, executable, runtimePackage] of supportedTargets) {
      assert.equal(
        resolveOssplateBinary({ platform, arch, baseDir: wrapperRoot, packagesBaseDir }),
        path.join(packagesBaseDir, runtimePackage, "bin", executable)
      );
    }
  } finally {
    fs.rmSync(packagesBaseDir, { recursive: true, force: true });
  }
});

test("resolveOssplateBinary names the missing runtime package clearly", async () => {
  const { resolveOssplateBinary } = await loadModule();
  assert.throws(
    () =>
      resolveOssplateBinary({
        platform: "linux",
        arch: "x64",
        baseDir: fs.mkdtempSync(path.join(os.tmpdir(), "ossplate-missing-runtime-"))
      }),
    /Missing runtime package @stefdevscore\/ossplate-linux-x64/
  );
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

test("top-level npm package excludes bundled runtime binaries and scaffold runtime binaries", () => {
  execFileSync("cargo", ["build"], {
    cwd: path.join(repoRoot, "core-rs"),
    stdio: "ignore"
  });
  execFileSync("node", [path.join(repoRoot, "scripts", "stage-distribution-assets.mjs")], {
    cwd: repoRoot,
    stdio: "ignore"
  });
  const tarball = packNpmPackage(wrapperRoot);
  const unpackDir = fs.mkdtempSync(path.join(os.tmpdir(), "ossplate-js-main-pack-"));

  try {
    extractTarball(tarball, unpackDir);
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

    assert.ok(packagedFiles.includes(path.join("bin", "ossplate.js")));
    assert.ok(
      !packagedFiles.some((file) => /^bin\/(darwin|linux|win32)-/.test(file)),
      "top-level package should not ship platform runtime binaries"
    );
    assert.ok(
      !packagedFiles.some((file) =>
        /^scaffold[\\/]wrapper-js[\\/]bin[\\/](darwin|linux|win32)-/.test(file)
      ),
      "scaffold should not ship nested JS runtime binaries"
    );
    assert.ok(
      !packagedFiles.some((file) =>
        file.startsWith(path.join("scaffold", "wrapper-py", "src", "ossplate", "bin"))
      ),
      "scaffold should not ship nested Python runtime binaries"
    );
  } finally {
    fs.rmSync(unpackDir, { recursive: true, force: true });
    fs.rmSync(tarball, { force: true });
  }
});

test("runtime package tarball contains exactly one target binary", () => {
  execFileSync("cargo", ["build"], {
    cwd: path.join(repoRoot, "core-rs"),
    stdio: "ignore"
  });

  const runtime = currentRuntimePackage();
  const packageDir = path.join(wrapperRoot, "platform-packages", runtime.packageFolder);
  const tarball = packNpmPackage(packageDir);
  const unpackDir = fs.mkdtempSync(path.join(os.tmpdir(), "ossplate-js-runtime-pack-"));

  try {
    extractTarball(tarball, unpackDir);
    const packagedFiles = listFiles(path.join(unpackDir, "package"));
    const runtimeFiles = packagedFiles.filter((file) => file.startsWith("bin/"));
    assert.deepEqual(runtimeFiles, [path.join("bin", runtime.executable)]);
  } finally {
    fs.rmSync(unpackDir, { recursive: true, force: true });
    fs.rmSync(tarball, { force: true });
  }
});

test("installed js package and matching runtime package can create from scaffold payload", () => {
  execFileSync("cargo", ["build"], {
    cwd: path.join(repoRoot, "core-rs"),
    stdio: "ignore"
  });
  execFileSync("node", [path.join(repoRoot, "scripts", "stage-distribution-assets.mjs")], {
    cwd: repoRoot,
    stdio: "ignore"
  });

  const runtime = currentRuntimePackage();
  const runtimePackageDir = path.join(wrapperRoot, "platform-packages", runtime.packageFolder);
  const mainTarball = packNpmPackage(wrapperRoot);
  const runtimeTarball = packNpmPackage(runtimePackageDir);
  const installDir = fs.mkdtempSync(path.join(os.tmpdir(), "ossplate-js-install-"));
  const targetDir = path.join(installDir, "created");

  try {
    fs.writeFileSync(
      path.join(installDir, "package.json"),
      `${JSON.stringify({ name: "ossplate-js-install-test", private: true }, null, 2)}\n`
    );

    execFileSync("npm", ["install", runtimeTarball, mainTarball], {
      cwd: installDir,
      stdio: "ignore"
    });

    const packagedTool =
      process.platform === "win32"
        ? path.join(installDir, "node_modules", ".bin", "ossplate.cmd")
        : path.join(installDir, "node_modules", ".bin", "ossplate");
    const directVersion = execFileSync(
      path.join(repoRoot, "core-rs", "target", "debug", runtime.executable),
      ["version"],
      {
        cwd: repoRoot,
        encoding: "utf8"
      }
    ).trim();
    const packagedVersion = execFileSync(packagedTool, ["version"], {
      cwd: installDir,
      encoding: "utf8"
    }).trim();
    assert.equal(packagedVersion, directVersion);

    execFileSync(packagedTool, ["create", targetDir], {
      cwd: installDir,
      stdio: "ignore"
    });
    const output = execFileSync(packagedTool, ["validate", "--path", targetDir, "--json"], {
      cwd: installDir,
      encoding: "utf8"
    }).trim();
    assert.equal(output, '{"ok":true,"issues":[]}');
  } finally {
    fs.rmSync(installDir, { recursive: true, force: true });
    fs.rmSync(mainTarball, { force: true });
    fs.rmSync(runtimeTarball, { force: true });
  }
});

function currentRuntimePackage() {
  const match = supportedTargets.find(
    ([platform, arch]) => platform === process.platform && arch === process.arch
  );
  if (!match) {
    throw new Error(`unsupported host platform for JS package tests: ${process.platform}/${process.arch}`);
  }
  const [_platform, _arch, target, executable, packageName, packageFolder] = match;
  return { target, executable, packageName, packageFolder };
}

function packNpmPackage(packageDir) {
  const tarballName = execFileSync("npm", ["pack"], {
    cwd: packageDir,
    encoding: "utf8"
  })
    .trim()
    .split("\n")
    .at(-1);
  return path.join(packageDir, tarballName);
}

function extractTarball(tarball, targetDir) {
  execFileSync("tar", ["-xzf", tarball, "-C", targetDir]);
}

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
