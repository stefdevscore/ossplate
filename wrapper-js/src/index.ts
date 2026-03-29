import { accessSync, constants } from "node:fs";
import { spawn } from "node:child_process";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { arch as runtimeArch, platform as runtimePlatform } from "node:os";

const __dirname = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);
const ENV_OVERRIDE = "OSSPLATE_BINARY";
const TEMPLATE_ROOT_ENV = "OSSPLATE_TEMPLATE_ROOT";
const TARGETS: Record<string, Record<string, { target: string; packageName: string }>> = {
  darwin: {
    arm64: { target: "darwin-arm64", packageName: "@stefdevscore/ossplate-darwin-arm64" },
    x64: { target: "darwin-x64", packageName: "@stefdevscore/ossplate-darwin-x64" }
  },
  linux: { x64: { target: "linux-x64", packageName: "@stefdevscore/ossplate-linux-x64" } },
  win32: { x64: { target: "win32-x64", packageName: "@stefdevscore/ossplate-win32-x64" } }
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
  const runtimePackage = TARGETS[platform]?.[arch];
  if (!runtimePackage) {
    throw new Error(`Unsupported platform/arch: ${platform}/${arch}`);
  }

  const executable = platform === "win32" ? "ossplate.exe" : "ossplate";
  const packagedPath = resolveRuntimePackageBinary(
    runtimePackage.packageName,
    executable,
    options.baseDir ?? join(__dirname, ".."),
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
