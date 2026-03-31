import { execFileSync } from "node:child_process";
import { cpSync, copyFileSync, mkdirSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const wrapperRoot = join(repoRoot, "wrapper-js");
const mode = process.argv[2] ?? "pack";

main();

function main() {
  switch (mode) {
    case "pack": {
      const outDir = resolve(process.argv[3] ?? mkdtempSync(join(tmpdir(), "ossplate-js-tarball-")));
      const tarball = packTopLevelPackage({ outDir });
      console.log(tarball);
      return;
    }
    case "dry-run-json":
      console.log(dryRunTopLevelPackage());
      return;
    default:
      throw new Error("usage: node scripts/package-js.mjs <pack [out-dir]|dry-run-json>");
  }
}

export function dryRunTopLevelPackage() {
  return withTempPackageRoot((packageRoot) =>
    execNpm(["pack", "--dry-run", "--json"], {
      cwd: packageRoot,
      encoding: "utf8"
    })
  );
}

export function packTopLevelPackage({ outDir }) {
  mkdirSync(outDir, { recursive: true });
  return withTempPackageRoot((packageRoot) => {
    const tarballName = execNpm(["pack", "--pack-destination", outDir], {
      cwd: packageRoot,
      encoding: "utf8"
    })
      .trim()
      .split("\n")
      .at(-1);
    return join(outDir, tarballName);
  });
}

function withTempPackageRoot(callback) {
  const tempRoot = mkdtempSync(join(tmpdir(), "ossplate-js-package-root-"));
  const packageRoot = join(tempRoot, "wrapper-js");

  try {
    stagePackageRoot(packageRoot);
    return callback(packageRoot);
  } finally {
    rmSync(tempRoot, { recursive: true, force: true });
  }
}

function stagePackageRoot(packageRoot) {
  cpSync(wrapperRoot, packageRoot, {
    recursive: true,
    filter(source) {
      const name = basename(source);
      if (source === wrapperRoot) {
        return true;
      }
      if (name === "node_modules" || name === "scaffold") {
        return false;
      }
      if (source.endsWith(".tgz")) {
        return false;
      }
      return true;
    }
  });

  execNode([join(repoRoot, "scripts", "stage-distribution-assets.mjs"), "scaffold-package", join(packageRoot, "scaffold")], {
    cwd: repoRoot
  });
  copyFileSync(join(repoRoot, "runtime-targets.json"), join(packageRoot, "runtime-targets.json"));
}

function execNode(args, options = {}) {
  return execFileSync("node", args, options);
}

function execNpm(args, options = {}) {
  if (process.platform === "win32") {
    return execFileSync(process.env.ComSpec ?? "cmd.exe", ["/d", "/s", "/c", "npm", ...args], options);
  }
  return execFileSync("npm", args, options);
}
