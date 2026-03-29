import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const { targets } = JSON.parse(readFileSync(join(repoRoot, "runtime-targets.json"), "utf8"));

export function getRuntimeTargets() {
  return targets;
}

export function runtimeTargetByName(target) {
  const spec = targets.find((entry) => entry.target === target);
  if (!spec) {
    throw new Error(`unsupported runtime package target: ${target}`);
  }
  return spec;
}

export function runtimePackageFolder(target) {
  return `ossplate-${runtimeTargetByName(target).folderSuffix}`;
}

export function runtimePackageName(rootPackageName, target) {
  return `${rootPackageName}-${runtimeTargetByName(target).packageSuffix}`;
}

export function resolveNodeHostTarget(platformName, archName) {
  const spec = targets.find(
    (entry) => entry.node.platform === platformName && entry.node.arch === archName
  );
  if (!spec) {
    throw new Error(`Unsupported host platform: ${platformName}/${archName}`);
  }
  return spec;
}

export function stagedRuntimeBinaryPath(root, target) {
  const spec = runtimeTargetByName(target);
  return join(root, ".dist-assets", "runtime", target, spec.binary);
}
