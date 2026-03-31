import { cpSync, existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { dirname, isAbsolute, join } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const outputArg = process.argv[2];
const outputRoot = outputArg
  ? isAbsolute(outputArg)
    ? outputArg
    : join(repoRoot, outputArg)
  : join(repoRoot, "core-rs", "embedded-template-root");

stageEmbeddedTemplate();

function stageEmbeddedTemplate() {
  rmSync(outputRoot, {
    force: true,
    recursive: true,
    maxRetries: 5,
    retryDelay: 50
  });
  mkdirSync(outputRoot, { recursive: true });

  const requiredPaths = new Set(["ossplate.toml", "scaffold-payload.json", "source-checkout.json"]);
  for (const manifestName of ["scaffold-payload.json", "source-checkout.json"]) {
    const manifest = JSON.parse(readFileSync(join(repoRoot, manifestName), "utf8"));
    for (const relativePath of manifest.requiredPaths) {
      if (relativePath.startsWith("core-rs/")) {
        continue;
      }
      requiredPaths.add(relativePath);
    }
  }

  for (const relativePath of [...requiredPaths].sort()) {
    const sourcePath = join(repoRoot, relativePath);
    if (!existsSync(sourcePath)) {
      throw new Error(`Required embedded template path is missing: ${relativePath}`);
    }

    const destinationPath = join(outputRoot, relativePath);
    mkdirSync(dirname(destinationPath), { recursive: true });
    cpSync(sourcePath, destinationPath, { recursive: true });
  }
}
