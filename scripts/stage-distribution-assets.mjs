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
import { dirname, join } from "node:path";
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
const excludedPrefixes = manifest.excludedPrefixes;
const runtimeTargets = getRuntimeTargets();

const wrapperTargets = [
  join(repoRoot, "wrapper-js", "scaffold"),
  join(repoRoot, "wrapper-py", "src", "ossplate", "scaffold")
];

const currentTarget = resolveCurrentTarget();
const currentBinaryName = currentTarget.binary;
const sourceBinary = join(repoRoot, "core-rs", "target", "debug", currentBinaryName);

const mode = process.argv[2] ?? "default";

if (mode === "runtime-package") {
  const target = process.argv[3];
  if (!target) {
    throw new Error("usage: node scripts/stage-distribution-assets.mjs runtime-package <target>");
  }
  stageRuntimePackage(process.cwd(), target);
} else {
  stageDefault();
}

function stageDefault() {
  for (const destinationRoot of wrapperTargets) {
    stageScaffold(destinationRoot);
  }

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

  for (const relativePath of requiredPaths) {
    const sourcePath = join(repoRoot, relativePath);
    if (!existsSync(sourcePath)) {
      throw new Error(`Required scaffold path is missing: ${relativePath}`);
    }

    const destinationPath = join(destinationRoot, relativePath);
    mkdirSync(dirname(destinationPath), { recursive: true });
    cpSync(sourcePath, destinationPath, { recursive: true });
  }
}

function stagePythonRuntime() {
  const relativePath = `wrapper-py/src/ossplate/bin/${currentTarget.target}/${currentTarget.binary}`;
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
  const expectedPackageName = runtimePackageName("ossplate", target);
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
    throw new Error(`required ossplate binary is missing at ${sourceBinary}`);
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
