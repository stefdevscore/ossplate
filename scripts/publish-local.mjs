import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { arch, platform } from "node:os";
import { fileURLToPath } from "node:url";

const scriptRepoRoot = fileURLToPath(new URL("..", import.meta.url));

if (isMainModule()) {
  try {
    runPublish(parseArgs(process.argv.slice(2)), createSystemContext());
  } catch (error) {
    console.error(`ossplate publish: ${error.message}`);
    process.exit(1);
  }
}

export function parseArgs(argv) {
  const options = {
    root: scriptRepoRoot,
    dryRun: false,
    registry: "all",
    skipExisting: false
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--root") {
      options.root = resolve(argv[++index] ?? "");
    } else if (arg === "--dry-run") {
      options.dryRun = true;
    } else if (arg === "--registry") {
      options.registry = argv[++index] ?? "";
    } else if (arg === "--skip-existing") {
      options.skipExisting = true;
    } else {
      throw new Error(`unsupported argument: ${arg}`);
    }
  }

  if (!["all", "npm", "pypi", "cargo"].includes(options.registry)) {
    throw new Error(`unsupported --registry value: ${options.registry}`);
  }

  return options;
}

export function runPublish(options, context) {
  const root = resolve(options.root);
  const metadata = loadMetadata(root);
  const host = resolveHostTarget(context);
  const completedRegistries = [];

  runPreflight(root, metadata, context);

  const registries = options.registry === "all" ? ["npm", "pypi", "cargo"] : [options.registry];
  try {
    for (const registry of registries) {
      if (registry === "npm") {
        publishNpm(root, metadata, host, options, context);
      } else if (registry === "pypi") {
        publishPypi(root, host, options, context);
      } else if (registry === "cargo") {
        publishCargo(root, metadata, options, context);
      } else {
        throw new Error(`unsupported registry: ${registry}`);
      }
      completedRegistries.push(registry);
    }
  } catch (error) {
    if (completedRegistries.length > 0) {
      error.message = `${error.message}\npartial publish completed before failure: ${completedRegistries.join(
        ", "
      )}\nfix the issue, then rerun the same command with --skip-existing or cut the next patch release.`;
    }
    throw error;
  }

  context.log(
    options.dryRun
      ? `publish dry-run complete (${registries.join(", ")})`
      : `publish complete (${registries.join(", ")})`
  );
}

function runPreflight(root, metadata, context) {
  const jsLockfileMode = context.npmVersionExists(metadata.rootPackage.name, metadata.version)
    ? "resolved"
    : "placeholder";
  const manifestPath = join(root, "core-rs", "Cargo.toml");

  context.run({
    label: "tool:validate",
    cwd: root,
    program: "cargo",
    args: ["run", "--quiet", "--manifest-path", manifestPath, "--", "validate", "--path", root, "--json"]
  });
  context.run({
    label: "tool:sync-check",
    cwd: root,
    program: "cargo",
    args: ["run", "--quiet", "--manifest-path", manifestPath, "--", "sync", "--path", root, "--check"]
  });
  context.run({
    label: "release:assert",
    cwd: root,
    program: "node",
    args: [join(root, "scripts", "assert-release-state.mjs")]
  });
  context.run({
    label: "js:lockfile-assert",
    cwd: root,
    program: "node",
    args: [join(root, "scripts", "assert-js-lockfile-state.mjs"), jsLockfileMode]
  });
  context.run({
    label: "publish:assert",
    cwd: root,
    program: "node",
    args: [join(root, "scripts", "assert-publish-readiness.mjs"), "publish"]
  });
}

function publishNpm(root, metadata, host, options, context) {
  const hostRuntime = metadata.runtimePackages.find((entry) => entry.target === host.target);
  if (!hostRuntime) {
    throw new Error(`no runtime package metadata found for host target ${host.target}`);
  }

  if (!options.dryRun) {
    const missingOtherRuntimes = metadata.runtimePackages
      .filter((entry) => entry.name !== hostRuntime.name)
      .filter((entry) => !context.npmVersionExists(entry.name, metadata.version))
      .map((entry) => `${entry.name}@${metadata.version}`);
    if (missingOtherRuntimes.length > 0) {
      throw new Error(
        `cannot publish top-level npm package until the other runtime packages exist:\n- ${missingOtherRuntimes.join(
          "\n- "
        )}\npublish them from matching host runners first`
      );
    }
  }

  context.run({
    label: "npm:build-core",
    cwd: root,
    program: "cargo",
    args: ["build", "--manifest-path", join(root, "core-rs", "Cargo.toml")]
  });

  const runtimeDir = join(root, "wrapper-js", "platform-packages", hostRuntime.folder);
  publishNpmPackage(runtimeDir, hostRuntime.name, metadata.version, options, context, "npm:runtime");
  const runtimePublishCompleted =
    options.dryRun || !(options.skipExisting && context.npmVersionExists(hostRuntime.name, metadata.version));

  try {
    const topLevelName = metadata.rootPackage.name;
    const wrapperJsDir = join(root, "wrapper-js");
    if (
      options.dryRun ||
      !(options.skipExisting && context.npmVersionExists(topLevelName, metadata.version))
    ) {
      if (!options.dryRun) {
        const runtimeNames = metadata.runtimePackages.map((entry) => entry.name);
        context.run({
          label: "npm:wait-runtime-versions",
          cwd: root,
          program: "node",
          args: [
            join(root, "scripts", "wait-for-npm-versions.mjs"),
            metadata.version,
            ...runtimeNames
          ]
        });
      }
      context.run({
        label: "npm:install-build-deps",
        cwd: wrapperJsDir,
        program: "npm",
        args: ["install", "--no-package-lock"]
      });
      context.run({
        label: "npm:build",
        cwd: wrapperJsDir,
        program: "npm",
        args: ["run", "build"]
      });
    }
    publishNpmPackage(wrapperJsDir, topLevelName, metadata.version, options, context, "npm:top-level");
  } catch (error) {
    if (runtimePublishCompleted) {
      error.message = `${error.message}\npartial publish completed before failure: npm`;
    }
    throw error;
  }
}

function publishNpmPackage(directory, name, version, options, context, labelPrefix) {
  if (!options.dryRun && options.skipExisting && context.npmVersionExists(name, version)) {
    context.log(`${labelPrefix}: skip ${name}@${version} (already published)`);
    return;
  }
  const args = ["publish", "--access", "public"];
  if (options.dryRun) {
    args.push("--dry-run");
  }
  context.run({
    label: `${labelPrefix}:publish`,
    cwd: directory,
    program: "npm",
    args
  });
}

function publishPypi(root, host, options, context) {
  const wrapperPyDir = join(root, "wrapper-py");
  const python = context.pythonCommand();
  context.run({
    label: "pypi:build-core",
    cwd: root,
    program: "cargo",
    args: ["build", "--manifest-path", join(root, "core-rs", "Cargo.toml")]
  });
  context.run({
    label: "pypi:install-build-tools",
    cwd: wrapperPyDir,
    program: python,
    args: ["-m", "pip", "install", "--upgrade", "pip", "build", "twine"]
  });
  context.run({
    label: "pypi:build-wheel",
    cwd: wrapperPyDir,
    program: python,
    args: ["-m", "build", "--wheel", "--outdir", join("dist", host.target)],
    env: { OSSPLATE_PY_TARGET: host.target }
  });
  context.run({
    label: "pypi:build-sdist",
    cwd: wrapperPyDir,
    program: python,
    args: ["-m", "build", "--sdist", "--outdir", join("dist", "sdist")]
  });

  const wheelPaths = collectFiles(join(wrapperPyDir, "dist", host.target), (name) => name.endsWith(".whl"));
  const sdistPaths = collectFiles(join(wrapperPyDir, "dist", "sdist"), (name) => name.endsWith(".tar.gz"));
  const artifactPaths = [...wheelPaths, ...sdistPaths];
  if (artifactPaths.length === 0) {
    throw new Error("PyPI build did not produce any artifacts");
  }

  if (options.dryRun) {
    context.run({
      label: "pypi:check",
      cwd: wrapperPyDir,
      program: python,
      args: ["-m", "twine", "check", ...artifactPaths]
    });
    return;
  }

  const uploadArgs = ["-m", "twine", "upload"];
  if (options.skipExisting) {
    uploadArgs.push("--skip-existing");
  }
  uploadArgs.push(...artifactPaths);
  context.run({
    label: "pypi:upload",
    cwd: wrapperPyDir,
    program: python,
    args: uploadArgs
  });
}

function publishCargo(root, metadata, options, context) {
  if (!options.dryRun && options.skipExisting && context.cargoVersionExists(metadata.cargoName, metadata.version)) {
    context.log(`cargo: skip ${metadata.cargoName}@${metadata.version} (already published)`);
    return;
  }

  const args = ["publish", "--manifest-path", join(root, "core-rs", "Cargo.toml")];
  if (options.dryRun) {
    args.splice(1, 0, "--dry-run");
  }
  context.run({
    label: "cargo:publish",
    cwd: root,
    program: "cargo",
    args
  });
}

function loadMetadata(root) {
  const rootPackage = readJson(join(root, "wrapper-js", "package.json"));
  const cargoToml = readText(join(root, "core-rs", "Cargo.toml"));
  const cargoNameMatch = cargoToml.match(/^name = "([^"]+)"$/m);
  const cargoVersionMatch = cargoToml.match(/^version = "([^"]+)"$/m);
  if (!cargoNameMatch || !cargoVersionMatch) {
    throw new Error("failed to read cargo package metadata");
  }
  const version = rootPackage.version;
  const runtimePackages = [
    { target: "darwin-arm64", folder: "ossplate-darwin-arm64", name: `${rootPackage.name}-darwin-arm64` },
    { target: "darwin-x64", folder: "ossplate-darwin-x64", name: `${rootPackage.name}-darwin-x64` },
    { target: "linux-x64", folder: "ossplate-linux-x64", name: `${rootPackage.name}-linux-x64` },
    { target: "win32-x64", folder: "ossplate-win32-x64", name: `${rootPackage.name}-windows-x64` }
  ];
  return {
    version,
    rootPackage,
    cargoName: cargoNameMatch[1],
    cargoVersion: cargoVersionMatch[1],
    runtimePackages
  };
}

function resolveHostTarget(context) {
  const target = {
    darwin: { arm64: "darwin-arm64", x64: "darwin-x64" },
    linux: { x64: "linux-x64" },
    win32: { x64: "win32-x64" }
  }[context.platform()]?.[context.arch()];

  if (!target) {
    throw new Error(`unsupported host platform for local publish: ${context.platform()}/${context.arch()}`);
  }
  return { target };
}

function collectFiles(directory, predicate) {
  if (!existsSync(directory)) {
    return [];
  }
  return readdirSync(directory)
    .filter(predicate)
    .sort()
    .map((name) => join(directory, name));
}

function readJson(path) {
  return JSON.parse(readText(path));
}

function readText(path) {
  return readFileSync(path, "utf8");
}

function createSystemContext() {
  return {
    run(command) {
      const env = { ...process.env, ...(command.env ?? {}) };
      execFileSync(command.program, command.args, {
        cwd: command.cwd,
        env,
        stdio: "inherit"
      });
    },
    npmVersionExists(packageName, version) {
      try {
        execNpm(["view", `${packageName}@${version}`, "version"], {
          cwd: scriptRepoRoot,
          stdio: "ignore"
        });
        return true;
      } catch {
        return false;
      }
    },
    cargoVersionExists(crateName, version) {
      try {
        execFileSync("curl", ["-fsSL", `https://crates.io/api/v1/crates/${crateName}/${version}`], {
          cwd: scriptRepoRoot,
          stdio: "ignore"
        });
        return true;
      } catch {
        return false;
      }
    },
    pythonCommand() {
      for (const candidate of ["python3.14", "python3.13", "python3.12", "python3.11", "python3.10", "python3", "python"]) {
        try {
          execFileSync(candidate, ["-c", "import sys; raise SystemExit(0 if sys.version_info >= (3, 10) else 1)"], {
            cwd: scriptRepoRoot,
            stdio: "ignore"
          });
          return candidate;
        } catch {
          continue;
        }
      }
      throw new Error("no Python 3.10+ interpreter found on PATH");
    },
    platform,
    arch,
    log(message) {
      console.log(message);
    }
  };
}

function execNpm(args, options = {}) {
  if (process.platform === "win32") {
    return execFileSync(process.env.ComSpec ?? "cmd.exe", ["/d", "/s", "/c", "npm", ...args], options);
  }
  return execFileSync("npm", args, options);
}

function isMainModule() {
  return process.argv[1] === fileURLToPath(import.meta.url);
}
