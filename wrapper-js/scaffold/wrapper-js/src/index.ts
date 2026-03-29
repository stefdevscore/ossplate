import { accessSync, constants, existsSync, readFileSync } from "node:fs";
import { spawn } from "node:child_process";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { arch as runtimeArch, platform as runtimePlatform } from "node:os";

const __dirname = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);
const ENV_OVERRIDE = "OSSPLATE_BINARY";
const TEMPLATE_ROOT_ENV = "OSSPLATE_TEMPLATE_ROOT";
type RuntimeTarget = {
  target: string;
  binary?: string;
  packageSuffix: string;
  node: { platform: string; arch: string };
};

function runtimeTargetsManifestPath(): string {
  const packageRoot = join(__dirname, "..");
  const scaffoldPath = join(packageRoot, "scaffold", "runtime-targets.json");
  const sourcePath = join(packageRoot, "..", "runtime-targets.json");
  return existsSync(scaffoldPath) ? scaffoldPath : sourcePath;
}

function loadRuntimeTargets(): RuntimeTarget[] {
  return JSON.parse(readFileSync(runtimeTargetsManifestPath(), "utf8")).targets as RuntimeTarget[];
}

function resolveHostRuntimeTarget(
  platform: NodeJS.Platform,
  arch: string,
  runtimeTargets: RuntimeTarget[]
): RuntimeTarget {
  const runtimeTarget = runtimeTargets.find(
    (entry) => entry.node.platform === platform && entry.node.arch === arch
  );
  if (!runtimeTarget) {
    throw new Error(`Unsupported platform/arch: ${platform}/${arch}`);
  }
  return runtimeTarget;
}

function executableNameForPlatform(platform: NodeJS.Platform): string {
  return platform === "win32" ? "ossplate.exe" : "ossplate";
}

function templateRootForBaseDir(baseDir: string): string {
  return join(baseDir, "scaffold");
}

type WrapperExecutionPlan = {
  binaryPath: string;
  templateRoot: string;
};

export function resolveOssplateBinary(
  options: {
    baseDir?: string;
    packagesBaseDir?: string;
    platform?: NodeJS.Platform;
    arch?: string;
  } = {}
): string {
  const envOverride = process.env[ENV_OVERRIDE];
  if (envOverride) {
    return envOverride;
  }

  const platform = options.platform ?? runtimePlatform();
  const arch = options.arch ?? runtimeArch();
  const baseDir = options.baseDir ?? join(__dirname, "..");
  const runtimePackage = resolveHostRuntimeTarget(platform, arch, loadRuntimeTargets());
  const executable = executableNameForPlatform(platform);
  const packagedPath = resolveRuntimePackageBinary(
    `ossplate-${runtimePackage.packageSuffix}`,
    executable,
    baseDir,
    options.packagesBaseDir
  );
  assertExecutable(packagedPath);
  return packagedPath;
}

function planOssplateExecution(
  options: { baseDir?: string; platform?: NodeJS.Platform; arch?: string } = {}
): WrapperExecutionPlan {
  const baseDir = options.baseDir ?? join(__dirname, "..");
  return {
    binaryPath: resolveOssplateBinary(options),
    templateRoot: templateRootForBaseDir(baseDir)
  };
}

function spawnOssplate(
  plan: WrapperExecutionPlan,
  args: string[],
  env: NodeJS.ProcessEnv = process.env
): void {
  const child = spawn(plan.binaryPath, args, {
    stdio: "inherit",
    env: {
      ...env,
      [TEMPLATE_ROOT_ENV]: env[TEMPLATE_ROOT_ENV] ?? plan.templateRoot
    }
  });

  child.on("exit", (code) => {
    process.exit(code ?? 0);
  });

  child.on("error", (error) => {
    console.error(`ossplate: ${error.message}`);
    process.exit(1);
  });
}

export function runOssplate(
  args: string[] = [],
  options: { baseDir?: string; platform?: NodeJS.Platform; arch?: string } = {}
): void {
  spawnOssplate(planOssplateExecution(options), args);
}

function assertExecutable(filePath: string): void {
  try {
    accessSync(filePath, constants.X_OK);
  } catch {
    throw new Error(`Bundled ossplate binary not found or not executable at ${filePath}`);
  }
}

function resolveRuntimePackageBinary(
  packageName: string,
  executable: string,
  baseDir: string,
  packagesBaseDir?: string
): string {
  if (packagesBaseDir) {
    return join(packagesBaseDir, packageName, "bin", executable);
  }

  try {
    const packageJsonPath = require.resolve(`${packageName}/package.json`, {
      paths: [baseDir]
    });
    return join(dirname(packageJsonPath), "bin", executable);
  } catch {
    throw new Error(
      `Missing runtime package ${packageName}. Reinstall ossplate so npm can install the matching platform package.`
    );
  }
}
