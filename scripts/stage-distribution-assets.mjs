import { execFileSync } from "node:child_process";
import {
  chmodSync,
  copyFileSync,
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync
} from "node:fs";
import { arch, platform } from "node:os";
import { dirname, isAbsolute, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  getRuntimeTargets,
  stagedRuntimeBinaryPath,
  resolveNodeHostTarget,
  runtimePackageFolder,
  runtimePackageName,
  runtimeTargetByName
} from "./runtime-targets.mjs";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const manifest = JSON.parse(
  readFileSync(join(repoRoot, "scaffold-payload.json"), "utf8")
);
const requiredPaths = manifest.requiredPaths;
const requiredManifestPaths = ["ossplate.toml", "scaffold-payload.json", "source-checkout.json"];
const excludedPrefixes = manifest.excludedPrefixes;
const runtimeTargets = getRuntimeTargets();
const rootPackage = JSON.parse(readFileSync(join(repoRoot, "wrapper-js", "package.json"), "utf8"));
const pythonPackageSrcDir = readPythonPackageSrcDir();

const currentTarget = resolveCurrentTarget();
const currentBinaryName = currentTarget.binary;
const sourceBinary = join(repoRoot, "core-rs", "target", "debug", currentBinaryName);

const mode = process.argv[2] ?? "default";

switch (mode) {
  case "runtime-package": {
    const target = process.argv[3];
    if (!target) {
      throw new Error("usage: node scripts/stage-distribution-assets.mjs runtime-package <target>");
    }
    stageRuntimePackage(process.cwd(), target);
    break;
  }
  case "runtime-artifact": {
    const target = process.argv[3];
    if (!target) {
      throw new Error(
        "usage: node scripts/stage-distribution-assets.mjs runtime-artifact <target>"
      );
    }
    stageRuntimeArtifact(target);
    break;
  }
  case "scaffold-package": {
    const destinationRoot = process.argv[3];
    if (!destinationRoot) {
      throw new Error(
        "usage: node scripts/stage-distribution-assets.mjs scaffold-package <destination-root>"
      );
    }
    stageScaffoldPackage(destinationRoot);
    break;
  }
  case "embedded-template": {
    stageEmbeddedTemplate(process.argv[3]);
    break;
  }
  case "default":
    stageDefault();
    break;
  default:
    throw new Error(
      "usage: node scripts/stage-distribution-assets.mjs <default|runtime-package|runtime-artifact|scaffold-package|embedded-template> [args]"
    );
}

function stageDefault() {
  cleanAllRuntimePackageBins();
  if (existsSync(sourceBinary)) {
    stageRuntimeArtifact(currentTarget.target);
    stagePythonRuntime();
    stageRuntimePackage(
      join(repoRoot, "wrapper-js", "platform-packages", runtimePackageFolder(currentTarget.target)),
      currentTarget.target
    );
  }
}

function stageScaffold(destinationRoot) {
  removeTree(destinationRoot);
  mkdirSync(destinationRoot, { recursive: true });

  for (const relativePath of [...new Set([...requiredManifestPaths, ...requiredPaths])]) {
    const sourcePath = join(repoRoot, relativePath);
    if (!existsSync(sourcePath)) {
      throw new Error(`Required scaffold path is missing: ${relativePath}`);
    }

    const destinationPath = join(destinationRoot, relativePath);
    mkdirSync(dirname(destinationPath), { recursive: true });
    cpSync(sourcePath, destinationPath, { recursive: true });
  }
}

function stageScaffoldPackage(destinationRoot) {
  stageScaffold(destinationRoot);
  stageEmbeddedTemplate(join(destinationRoot, "core-rs", "embedded-template-root"));
}

function stageEmbeddedTemplate(outputRoot = "core-rs/generated-embedded-template-root") {
  const target = isAbsolute(outputRoot) ? outputRoot : join(repoRoot, outputRoot);
  execNode(["scripts/stage-embedded-template.mjs", target]);
}

function stagePythonRuntime() {
  const relativePath = `wrapper-py/${pythonPackageSrcDir}/bin/${currentTarget.target}/${currentTarget.binary}`;
  const destination = join(repoRoot, relativePath);
  mkdirSync(dirname(destination), { recursive: true });
  copyFileSync(stagedRuntimeBinaryPath(repoRoot, currentTarget.target), destination);
  chmodSync(destination, 0o755);
}

function cleanAllRuntimePackageBins() {
  for (const target of runtimeTargets) {
    removeTree(
      join(repoRoot, "wrapper-js", "platform-packages", runtimePackageFolder(target.target), "bin")
    );
  }
}

function stageRuntimePackage(packageRoot, target) {
  const spec = runtimeTargetByName(target);
  const expectedPackageFolder = runtimePackageFolder(target);
  const expectedPackageName = runtimePackageName(rootPackage.name, target);
  const expectedPackageSuffix = `/${expectedPackageFolder}`;
  if (target !== currentTarget.target) {
    throw new Error(
      `cannot stage ${target} from host ${currentTarget.target}; use the matching runner for this runtime package`
    );
  }
  const stagedBinary = stageRuntimeArtifact(target);

  const packageJsonPath = join(packageRoot, "package.json");
  if (!existsSync(packageJsonPath)) {
    throw new Error(`runtime package root missing package.json: ${packageRoot}`);
  }
  const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf8"));
  if (
    packageJson.name !== expectedPackageName &&
    !packageJson.name.endsWith(expectedPackageSuffix)
  ) {
    throw new Error(
      `runtime package ${packageRoot} does not match target ${target}: expected package name ${expectedPackageName} or a name ending with ${expectedPackageFolder}, found ${packageJson.name}`
    );
  }

  const executable = spec.binary;
  const destination = join(packageRoot, "bin", executable);
  removeTree(join(packageRoot, "bin"));
  mkdirSync(dirname(destination), { recursive: true });
  copyFileSync(stagedBinary, destination);
  chmodSync(destination, 0o755);
}

function stageRuntimeArtifact(target) {
  if (target !== currentTarget.target) {
    throw new Error(
      `cannot stage ${target} from host ${currentTarget.target}; use the matching runner for this runtime package`
    );
  }
  if (!existsSync(sourceBinary)) {
    throw new Error(`required CLI binary is missing at ${sourceBinary}`);
  }

  const destination = stagedRuntimeBinaryPath(repoRoot, target);
  mkdirSync(dirname(destination), { recursive: true });
  copyFileSync(sourceBinary, destination);
  chmodSync(destination, 0o755);
  return destination;
}

function removeTree(path) {
  rmSync(path, {
    force: true,
    recursive: true,
    maxRetries: 5,
    retryDelay: 50
  });
}

function resolveCurrentTarget() {
  return resolveNodeHostTarget(platform(), arch());
}

export function getScaffoldManifest() {
  return { requiredPaths, excludedPrefixes };
}

function readPythonPackageSrcDir() {
  const pyproject = readFileSync(join(repoRoot, "wrapper-py", "pyproject.toml"), "utf8");
  const match = pyproject.match(/packages\s*=\s*\[\s*"([^"]+)"\s*\]/);
  if (!match) {
    throw new Error("wrapper-py/pyproject.toml is missing a wheel packages entry");
  }
  return match[1];
}

function execNode(args) {
  if (process.platform === "win32") {
    execFileSync(process.env.ComSpec ?? "cmd.exe", ["/d", "/s", "/c", "node", ...args], {
      cwd: repoRoot,
      stdio: "inherit"
    });
    return;
  }
  execFileSync("node", args, {
    cwd: repoRoot,
    stdio: "inherit"
  });
}
