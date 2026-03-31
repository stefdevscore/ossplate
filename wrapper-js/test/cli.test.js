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
  fs.readFileSync(path.join(repoRoot, "scaffold-payload.json"), "utf8")
);
const packageJson = JSON.parse(
  fs.readFileSync(path.join(wrapperRoot, "package.json"), "utf8")
);
const pyproject = fs.readFileSync(path.join(repoRoot, "wrapper-py", "pyproject.toml"), "utf8");
const runtimeTargets = JSON.parse(
  fs.readFileSync(path.join(repoRoot, "runtime-targets.json"), "utf8")
).targets;
const wrapperCommand = Object.keys(packageJson.bin)[0];
const wrapperBinScript = packageJson.bin[wrapperCommand];
const pythonPackageSrcDir =
  pyproject.match(/packages\s*=\s*\[\s*"([^"]+)"\s*\]/)?.[1] ?? "src/ossplate";

const supportedTargets = runtimeTargets.map((entry) => [
  entry.node.platform,
  entry.node.arch,
  entry.target,
  entry.binary,
  `${packageJson.name}-${entry.packageSuffix}`,
  `ossplate-${entry.folderSuffix}`
]);

async function loadModule() {
  return import(pathToFileURL(distModule).href);
}

test("env override takes precedence for wrapper execution", () => {
  const output = execFileSync("node", [wrapperBinScript, "version"], {
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

test("buildChildEnv forwards only the wrapper contract plus selected passthrough vars", async () => {
  const { buildChildEnv } = await loadModule();
  const env = buildChildEnv(
    {
      binaryPath: "/tmp/ossplate",
      templateRoot: "/tmp/scaffold"
    },
    {
      PATH: "/usr/bin",
      HOME: "/tmp/home",
      NPM_TOKEN: "npm-secret",
      OSSPLATE_NPM_WAIT_ATTEMPTS: "12",
      OSSPLATE_TEMPLATE_ROOT: "/custom/scaffold",
      AWS_SECRET_ACCESS_KEY: "should-not-forward"
    }
  );

  assert.equal(env.PATH, "/usr/bin");
  assert.equal(env.HOME, "/tmp/home");
  assert.equal(env.NPM_TOKEN, "npm-secret");
  assert.equal(env.OSSPLATE_NPM_WAIT_ATTEMPTS, "12");
  assert.equal(env.OSSPLATE_TEMPLATE_ROOT, "/custom/scaffold");
  assert.equal("AWS_SECRET_ACCESS_KEY" in env, false);
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
  const packagesBaseDir = fs.mkdtempSync(path.join(os.tmpdir(), "ossplate-missing-runtime-"));
  assert.throws(
    () =>
      resolveOssplateBinary({
        platform: "linux",
        arch: "x64",
        baseDir: wrapperRoot,
        packagesBaseDir
      }),
    new RegExp(`Missing runtime package ${packageJson.name}-linux-x64`)
  );
  fs.rmSync(packagesBaseDir, { recursive: true, force: true });
});

test("js wrapper matches the rust contract via env override", () => {
  prepareRustBuild();
  const coreBinary = path.join(repoRoot, "core-rs", "target", "debug", currentRuntimePackage().executable);
  for (const args of [
    ["version"],
    ["validate", "--path", repoRoot, "--json"],
    ["sync", "--path", repoRoot, "--check"]
  ]) {
    const direct = execFileSync(coreBinary, args, {
      cwd: repoRoot,
      encoding: "utf8"
    }).trim();
    const wrapped = execFileSync("node", [wrapperBinScript, ...args], {
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
  prepareRustBuild();
  execFileSync(
    "node",
    [path.join(repoRoot, "scripts", "stage-distribution-assets.mjs"), "runtime-artifact", currentRuntimePackage().target],
    {
      cwd: repoRoot,
      stdio: "ignore"
    }
  );
  ensureGeneratedEmbeddedTemplate();
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
    assert.ok(packagedFiles.includes("runtime-targets.json"));
    for (const excludedPrefix of scaffoldManifest.excludedPrefixes) {
      assert.ok(
        !packagedFiles.some((file) => file.startsWith(path.join("scaffold", excludedPrefix))),
        `unexpected packaged scaffold file under ${excludedPrefix}`
      );
    }

    assert.ok(packagedFiles.includes(wrapperBinScript));
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
        file.startsWith(path.join("scaffold", "wrapper-py", pythonPackageSrcDir, "bin"))
      ),
      "scaffold should not ship nested Python runtime binaries"
    );
  } finally {
    fs.rmSync(unpackDir, { recursive: true, force: true });
    fs.rmSync(tarball, { force: true });
  }
});

test("runtime package tarball contains exactly one target binary", () => {
  prepareRustBuild();

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

test("staging creates a neutral runtime artifact for the current host", () => {
  prepareRustBuild();
  execFileSync(
    "node",
    [path.join(repoRoot, "scripts", "stage-distribution-assets.mjs"), "runtime-artifact", currentRuntimePackage().target],
    {
      cwd: repoRoot,
      stdio: "ignore"
    }
  );
  ensureGeneratedEmbeddedTemplate();

  const runtime = currentRuntimePackage();
  const stagedBinary = path.join(repoRoot, ".dist-assets", "runtime", runtime.target, runtime.executable);
  assert.ok(fs.existsSync(stagedBinary), `expected staged runtime binary at ${stagedBinary}`);
});

test("installed js package and matching runtime package can create from scaffold payload", () => {
  prepareRustBuild();
  execFileSync(
    "node",
    [path.join(repoRoot, "scripts", "stage-distribution-assets.mjs"), "runtime-artifact", currentRuntimePackage().target],
    {
      cwd: repoRoot,
      stdio: "ignore"
    }
  );
  ensureGeneratedEmbeddedTemplate();

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
        ? path.join(installDir, "node_modules", ".bin", `${wrapperCommand}.cmd`)
        : path.join(installDir, "node_modules", ".bin", wrapperCommand);
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
      encoding: "utf8",
      env: sanitizedEnv()
    }).trim();
    assert.equal(packagedVersion, directVersion);

    execFileSync(packagedTool, ["create", targetDir], {
      cwd: installDir,
      stdio: "ignore",
      env: sanitizedEnv()
    });
    const output = execFileSync(packagedTool, ["validate", "--path", targetDir, "--json"], {
      cwd: installDir,
      encoding: "utf8",
      env: sanitizedEnv()
    }).trim();
    const parsed = JSON.parse(output);
    assert.equal(parsed.ok, true);
    assert.deepEqual(parsed.issues, []);
    assert.ok(Array.isArray(parsed.warnings));
    assert.ok(parsed.warnings.length >= 4);
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

function prepareRustBuild() {
  ensureGeneratedEmbeddedTemplate();
  execFileSync("cargo", ["build"], {
    cwd: path.join(repoRoot, "core-rs"),
    stdio: "ignore"
  });
}

function sanitizedEnv() {
  const env = { ...process.env };
  delete env.OSSPLATE_TEMPLATE_ROOT;
  return env;
}

function ensureGeneratedEmbeddedTemplate() {
  const generatedRoot = path.join(repoRoot, "core-rs", "generated-embedded-template-root");
  if (fs.existsSync(generatedRoot)) {
    return;
  }
  execFileSync("node", [path.join(repoRoot, "scripts", "stage-distribution-assets.mjs"), "embedded-template"], {
    cwd: repoRoot,
    stdio: "inherit"
  });
}

function packNpmPackage(packageDir) {
  if (path.resolve(packageDir) === wrapperRoot) {
    return execFileSync("node", [path.join(repoRoot, "scripts", "package-js.mjs"), "pack"], {
      cwd: repoRoot,
      encoding: "utf8"
    })
      .trim()
      .split("\n")
      .at(-1);
  }
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
