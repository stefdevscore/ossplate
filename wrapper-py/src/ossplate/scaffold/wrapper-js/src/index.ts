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
  packageSuffix: string;
  node: { platform: string; arch: string };
};

function loadRuntimeTargets(baseDir: string): RuntimeTarget[] {
  const packageRoot = join(__dirname, "..");
  const scaffoldPath = join(packageRoot, "scaffold", "runtime-targets.json");
  const sourcePath = join(packageRoot, "..", "runtime-targets.json");
  const manifestPath = existsSync(scaffoldPath) ? scaffoldPath : sourcePath;
  return JSON.parse(readFileSync(manifestPath, "utf8")).targets as RuntimeTarget[];
}

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
  const runtimePackage = loadRuntimeTargets(baseDir).find(
    (entry) => entry.node.platform === platform && entry.node.arch === arch
  );
  if (!runtimePackage) {
    throw new Error(`Unsupported platform/arch: ${platform}/${arch}`);
  }

  const executable = platform === "win32" ? "ossplate.exe" : "ossplate";
  const packagedPath = resolveRuntimePackageBinary(
    `ossplate-${runtimePackage.packageSuffix}`,
    executable,
    baseDir,
    options.packagesBaseDir
  );
  assertExecutable(packagedPath);
  return packagedPath;
}

export function runOssplate(
  args: string[] = [],
  options: { baseDir?: string; platform?: NodeJS.Platform; arch?: string } = {}
): void {
  const binPath = resolveOssplateBinary(options);
  const baseDir = options.baseDir ?? join(__dirname, "..");
  const child = spawn(binPath, args, {
    stdio: "inherit",
    env: {
      ...process.env,
      [TEMPLATE_ROOT_ENV]:
        process.env[TEMPLATE_ROOT_ENV] ?? join(baseDir, "scaffold")
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
