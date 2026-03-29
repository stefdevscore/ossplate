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

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const manifest = JSON.parse(
  readFileSync(join(repoRoot, "scaffold-manifest.json"), "utf8")
);
const requiredPaths = manifest.requiredPaths;
const excludedPrefixes = manifest.excludedPrefixes;
const runtimePackageFolders = {
  "darwin-arm64": "ossplate-darwin-arm64",
  "darwin-x64": "ossplate-darwin-x64",
  "linux-x64": "ossplate-linux-x64",
  "win32-x64": "ossplate-win32-x64"
};

const wrapperTargets = [
  join(repoRoot, "wrapper-js", "scaffold"),
  join(repoRoot, "wrapper-py", "src", "ossplate", "scaffold")
];

const currentTarget = resolveCurrentTarget();
const currentBinaryName = currentTarget.platform === "win32" ? "ossplate.exe" : "ossplate";
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
    stagePythonRuntime();
    stageRuntimePackage(
      join(repoRoot, "wrapper-js", "platform-packages", runtimePackageFolders[currentTarget.folder]),
      currentTarget.folder
    );
  }
}

function stageScaffold(destinationRoot) {
  rmSync(destinationRoot, { force: true, recursive: true });
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
  const relativePath = currentTarget.platform === "win32"
    ? `wrapper-py/src/ossplate/bin/${currentTarget.folder}/ossplate.exe`
    : `wrapper-py/src/ossplate/bin/${currentTarget.folder}/ossplate`;
  const destination = join(repoRoot, relativePath);
  mkdirSync(dirname(destination), { recursive: true });
  copyFileSync(sourceBinary, destination);
  chmodSync(destination, 0o755);
}

function cleanAllRuntimePackageBins() {
  for (const packageFolder of Object.values(runtimePackageFolders)) {
    rmSync(join(repoRoot, "wrapper-js", "platform-packages", packageFolder, "bin"), {
      force: true,
      recursive: true
    });
  }
}

function stageRuntimePackage(packageRoot, target) {
  const expectedPackageFolder = runtimePackageFolders[target];
  const expectedPackageSuffix = `/${expectedPackageFolder}`;
  if (!expectedPackageFolder) {
    throw new Error(`unsupported runtime package target: ${target}`);
  }
  if (target !== currentTarget.folder) {
    throw new Error(
      `cannot stage ${target} from host ${currentTarget.folder}; use the matching runner for this runtime package`
    );
  }
  if (!existsSync(sourceBinary)) {
    throw new Error(`required ossplate binary is missing at ${sourceBinary}`);
  }

  const packageJsonPath = join(packageRoot, "package.json");
  if (!existsSync(packageJsonPath)) {
    throw new Error(`runtime package root missing package.json: ${packageRoot}`);
  }
  const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf8"));
  if (
    packageJson.name !== expectedPackageFolder &&
    !packageJson.name.endsWith(expectedPackageSuffix)
  ) {
    throw new Error(
      `runtime package ${packageRoot} does not match target ${target}: expected name ending with ${expectedPackageFolder}, found ${packageJson.name}`
    );
  }

  const executable = target === "win32-x64" ? "ossplate.exe" : "ossplate";
  const destination = join(packageRoot, "bin", executable);
  rmSync(join(packageRoot, "bin"), { force: true, recursive: true });
  mkdirSync(dirname(destination), { recursive: true });
  copyFileSync(sourceBinary, destination);
  chmodSync(destination, 0o755);
}

function resolveCurrentTarget() {
  const platformName = platform();
  const archName = arch();
  const folder = {
    darwin: { arm64: "darwin-arm64", x64: "darwin-x64" },
    linux: { x64: "linux-x64" },
    win32: { x64: "win32-x64" }
  }[platformName]?.[archName];

  if (!folder) {
    throw new Error(`Unsupported host platform for staging: ${platformName}/${archName}`);
  }

  return { platform: platformName, folder };
}

export function getScaffoldManifest() {
  return { requiredPaths, excludedPrefixes };
}
