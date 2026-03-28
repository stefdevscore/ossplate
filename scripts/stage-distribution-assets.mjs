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

const wrapperTargets = [
  join(repoRoot, "wrapper-js", "scaffold"),
  join(repoRoot, "wrapper-py", "src", "ossplate", "scaffold")
];

const currentTarget = resolveCurrentTarget();
const currentBinaryName = currentTarget.platform === "win32" ? "ossplate.exe" : "ossplate";
const sourceBinary = join(repoRoot, "core-rs", "target", "debug", currentBinaryName);

for (const destinationRoot of wrapperTargets) {
  stageScaffold(destinationRoot);
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

  if (existsSync(sourceBinary)) {
    stageBinary(destinationRoot);
  }
}

function stageBinary(destinationRoot) {
  const relativePath = currentTarget.platform === "win32"
    ? `wrapper-py/src/ossplate/bin/${currentTarget.folder}/ossplate.exe`
    : `wrapper-py/src/ossplate/bin/${currentTarget.folder}/ossplate`;
  const targets = [
    join(repoRoot, "wrapper-js", "bin", currentTarget.folder, currentBinaryName),
    join(repoRoot, relativePath),
    join(destinationRoot, `wrapper-js/bin/${currentTarget.folder}/${currentBinaryName}`),
    join(destinationRoot, relativePath)
  ];

  for (const destination of targets) {
    mkdirSync(dirname(destination), { recursive: true });
    copyFileSync(sourceBinary, destination);
    chmodSync(destination, 0o755);
  }
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
