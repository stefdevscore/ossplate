import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { join, relative } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { getRuntimeTargets, runtimePackageFolder, runtimePackageName } from "./runtime-targets.mjs";

export const repoRoot = fileURLToPath(new URL("..", import.meta.url));

export function readText(relativePath) {
  return readFileSync(join(repoRoot, relativePath), "utf8");
}

export function readJson(relativePath) {
  return JSON.parse(readText(relativePath));
}

export function readRootPackage() {
  return readJson("wrapper-js/package.json");
}

export function readPyproject() {
  return readText("wrapper-py/pyproject.toml");
}

export function readScaffoldPayload() {
  return readJson("scaffold-payload.json");
}

export function readCargoVersion() {
  return readTomlSectionValue(readText("core-rs/Cargo.toml"), "package", "version", "core-rs/Cargo.toml");
}

export function readPyprojectVersion() {
  return readTomlSectionValue(
    readText("wrapper-py/pyproject.toml"),
    "project",
    "version",
    "wrapper-py/pyproject.toml"
  );
}

export function readVersions(rootPackage = readRootPackage()) {
  return {
    rust: readCargoVersion(),
    js: rootPackage.version,
    python: readPyprojectVersion()
  };
}

export function getExpectedOptionalDependencies(rootPackage = readRootPackage()) {
  return Object.fromEntries(
    getRuntimeTargets().map((entry) => [runtimePackageName(rootPackage.name, entry.target), rootPackage.version])
  );
}

export function getRuntimePackageNames(rootPackage = readRootPackage()) {
  return Object.keys(rootPackage.optionalDependencies ?? {});
}

export function getSupportedRuntimePackageNames(rootPackage = readRootPackage()) {
  return new Set(
    getRuntimeTargets().map((entry) => runtimePackageName(rootPackage.name, entry.target))
  );
}

export function assertVersionsAligned(versions) {
  assertAllEqual(Object.entries(versions), "package versions must stay aligned");
}

export function assertOptionalDependencies(rootPackage = readRootPackage()) {
  const actual = rootPackage.optionalDependencies ?? {};
  const expected = getExpectedOptionalDependencies(rootPackage);
  assertDeepEqual(
    actual,
    expected,
    "wrapper-js/package.json optionalDependencies must match the runtime package contract"
  );
}

export function assertRuntimePackages(rootPackage = readRootPackage()) {
  for (const entry of getRuntimeTargets()) {
    const folder = runtimePackageFolder(entry.target);
    const packageJson = readJson(join("wrapper-js", "platform-packages", folder, "package.json"));
    if (packageJson.name !== runtimePackageName(rootPackage.name, entry.target)) {
      fail(
        `runtime package ${folder} has name ${packageJson.name}; expected ${runtimePackageName(rootPackage.name, entry.target)}`
      );
    }
    if (packageJson.version !== rootPackage.version) {
      fail(
        `runtime package ${packageJson.name} has version ${packageJson.version}; expected ${rootPackage.version}`
      );
    }
  }
}

export function assertNoTrackedGeneratedBinaries() {
  const pythonPackageDir = readPythonPackageSrcDir();
  const pythonBinaryPattern = new RegExp(
    `^wrapper-py/${escapeRegExp(pythonPackageDir)}/bin/[^/]+/[^/]+$`
  );
  const tracked = listTrackedFiles()
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean)
    .filter(
      (line) =>
        /^wrapper-js\/platform-packages\/[^/]+\/bin\//.test(line) ||
        pythonBinaryPattern.test(line)
    );
  if (tracked.length > 0) {
    fail(
      `generated runtime binaries must not be tracked:\n${tracked
        .map((entry) => `- ${entry}`)
        .join("\n")}`
    );
  }
}

export function scaffoldMirrorRoots(root = repoRoot) {
  const pythonPackageDir = readPythonPackageSrcDir(root);
  return [
    join(root, "wrapper-js", "scaffold"),
    join(root, "wrapper-py", pythonPackageDir, "scaffold")
  ];
}

export function assertScaffoldSnapshots(
  scaffoldPayload = readScaffoldPayload(),
  {
    root = repoRoot,
    scaffoldRoots = scaffoldMirrorRoots(root)
  } = {}
) {
  for (const relativePath of scaffoldPayload.requiredPaths) {
    const source = join(root, relativePath);
    if (!existsSync(source)) {
      fail(`missing required scaffold source path ${relativePath}`);
    }
    const sourceContent = readFileSync(source);
    for (const scaffoldRoot of scaffoldRoots) {
      const snapshotPath = join(scaffoldRoot, relativePath);
      if (!existsSync(snapshotPath)) {
        fail(`missing scaffold snapshot ${relative(scaffoldRoot, snapshotPath)}`);
      }
      const snapshotContent = readFileSync(snapshotPath);
      if (!sourceContent.equals(snapshotContent)) {
        fail(`scaffold snapshot drift detected for ${relativePath}`);
      }
    }
  }
}

export function assertGeneratedScaffoldAssets(
  scaffoldPayload = readScaffoldPayload(),
  {
    root = repoRoot,
    pythonPackageSrcDir = readPythonPackageSrcDir(root),
    stageScaffoldPackage = defaultStageScaffoldPackage
  } = {}
) {
  const tempRoot = mkdtempSync(join(tmpdir(), "ossplate-scaffold-assets-"));
  const scaffoldRoots = [
    join(tempRoot, "wrapper-js", "scaffold"),
    join(tempRoot, "wrapper-py", pythonPackageSrcDir, "scaffold")
  ];

  try {
    for (const scaffoldRoot of scaffoldRoots) {
      stageScaffoldPackage(root, scaffoldRoot);
      const embeddedTemplateRoot = join(scaffoldRoot, "core-rs", "embedded-template-root");
      if (!existsSync(join(embeddedTemplateRoot, "ossplate.toml"))) {
        fail(`generated scaffold package is missing ${relative(scaffoldRoot, join(embeddedTemplateRoot, "ossplate.toml"))}`);
      }
    }
    assertScaffoldSnapshots(scaffoldPayload, { root, scaffoldRoots });
  } finally {
    rmSync(tempRoot, { recursive: true, force: true });
  }
}

export function assertNoRepoPackagingLeakage(root = repoRoot) {
  const pythonPackageSrcDir = readPythonPackageSrcDir(root);
  const leakedRoots = [
    join(root, "scaffold"),
    join(root, "src"),
    join(root, "wrapper-js", "scaffold"),
    join(root, "wrapper-py", pythonPackageSrcDir, "scaffold")
  ].filter((entry) => existsSync(entry));

  if (leakedRoots.length > 0) {
    fail(
      `package assembly must not leave repo-local scaffold roots behind:\n${leakedRoots
        .map((entry) => `- ${relative(root, entry)}`)
        .join("\n")}`
    );
  }
}

export function assertTopLevelPackShape() {
  const pythonBinaryPrefix = `scaffold/wrapper-py/${readPythonPackageSrcDir()}/bin/`;
  const output = execFileSync("node", [join(repoRoot, "scripts", "package-js.mjs"), "dry-run-json"], {
    cwd: repoRoot,
    encoding: "utf8"
  });
  const parsed = JSON.parse(output);
  const files = parsed[0]?.files?.map((entry) => entry.path) ?? [];
  if (!files.includes("runtime-targets.json")) {
    fail("top-level npm package is missing runtime-targets.json");
  }
  if (!files.includes("scaffold/ossplate.toml")) {
    fail("top-level npm package is missing scaffold/ossplate.toml");
  }
  for (const file of files) {
    if (/^bin\/(darwin|linux|win32)-/.test(file)) {
      fail(`top-level npm package still contains bundled runtime binary path ${file}`);
    }
    if (/^scaffold\/wrapper-js\/bin\/(darwin|linux|win32)-/.test(file)) {
      fail(`scaffold still contains nested JS runtime binary path ${file}`);
    }
    if (file.startsWith(pythonBinaryPrefix)) {
      fail(`scaffold still contains nested Python runtime binary path ${file}`);
    }
  }
}

export function assertRuntimePackageNames(rootPackage = readRootPackage()) {
  const runtimePackages = getRuntimePackageNames(rootPackage);
  const supportedRuntimePackages = getSupportedRuntimePackageNames(rootPackage);
  for (const packageName of runtimePackages) {
    if (!supportedRuntimePackages.has(packageName)) {
      fail(`runtime package ${packageName} does not match the supported npm runtime package contract`);
    }
  }
}

export function assertNpmVersionState({
  mode,
  version,
  rootPackage = readRootPackage(),
  npmVersionExists = defaultNpmVersionExists
}) {
  const runtimePackages = getRuntimePackageNames(rootPackage);
  const allNpmPackages = [rootPackage.name, ...runtimePackages];
  const states = new Map(allNpmPackages.map((name) => [name, npmVersionExists(name, version)]));
  const runtimeStates = runtimePackages.map((name) => states.get(name));
  const topLevelExists = states.get(rootPackage.name);

  if (mode === "release") {
    if ([...states.values()].some(Boolean)) {
      const published = [...states.entries()]
        .filter(([, exists]) => exists)
        .map(([name]) => `${name}@${version}`);
      fail(
        `release preflight requires a clean npm version state; already published package versions detected:\n- ${published.join("\n- ")}`
      );
    }
    return;
  }

  const allRuntimeExist = runtimeStates.every(Boolean);
  if (topLevelExists && !allRuntimeExist) {
    fail(`top-level package ${rootPackage.name}@${version} exists without all runtime packages`);
  }
}

export function assertReleaseState(rootPackage = readRootPackage()) {
  assertVersionsAligned(readVersions(rootPackage));
  assertOptionalDependencies(rootPackage);
  assertRuntimePackages(rootPackage);
  assertNoTrackedGeneratedBinaries();
  assertGeneratedScaffoldAssets(readScaffoldPayload());
  assertTopLevelPackShape();
  assertNoRepoPackagingLeakage();
}

export function assertPublishReadiness(mode, version, rootPackage = readRootPackage()) {
  assertRuntimePackageNames(rootPackage);
  assertNpmVersionState({ mode, version, rootPackage });
}

export function defaultNpmVersionExists(packageName, packageVersion) {
  try {
    execNpm(["view", `${packageName}@${packageVersion}`, "version"], {
      cwd: repoRoot,
      stdio: "ignore"
    });
    return true;
  } catch {
    return false;
  }
}

function execGit(args) {
  return execFileSync("git", args, {
    cwd: repoRoot,
    encoding: "utf8"
  }).trim();
}

function listTrackedFiles() {
  try {
    return execGit(["ls-files"]);
  } catch (error) {
    if (isNotGitRepositoryError(error)) {
      return "";
    }
    throw error;
  }
}

function defaultStageScaffoldPackage(root, destinationRoot) {
  execFileSync("node", [join(root, "scripts", "stage-distribution-assets.mjs"), "scaffold-package", destinationRoot], {
    cwd: root,
    stdio: "ignore"
  });
}

function npmCommand() {
  return process.platform === "win32" ? "npm.cmd" : "npm";
}

function execNpm(args, options = {}) {
  if (process.platform === "win32") {
    return execFileSync(process.env.ComSpec ?? "cmd.exe", ["/d", "/s", "/c", "npm", ...args], options);
  }
  return execFileSync(npmCommand(), args, options);
}

export function readTomlSectionValue(content, sectionName, key, fileLabel) {
  const sectionPattern = new RegExp(`^\\[${escapeRegExp(sectionName)}\\]\\s*$`, "m");
  const sectionMatch = sectionPattern.exec(content);
  if (!sectionMatch) {
    fail(`failed to read ${key} from ${fileLabel}`);
  }

  const sectionStart = sectionMatch.index + sectionMatch[0].length;
  const nextSectionOffset = content.slice(sectionStart).search(/^\[[^\]]+\]\s*$/m);
  const sectionBody =
    nextSectionOffset === -1
      ? content.slice(sectionStart)
      : content.slice(sectionStart, sectionStart + nextSectionOffset);
  const valuePattern = new RegExp(`^${escapeRegExp(key)}\\s*=\\s*"([^"]+)"\\s*$`, "m");
  const valueMatch = valuePattern.exec(sectionBody);
  if (!valueMatch) {
    fail(`failed to read ${key} from ${fileLabel}`);
  }
  return valueMatch[1];
}

function readPythonPackageSrcDir(root = repoRoot) {
  const pyproject = readFileSync(join(root, "wrapper-py", "pyproject.toml"), "utf8");
  const match = pyproject.match(/packages\s*=\s*\[\s*"([^"]+)"\s*\]/);
  if (!match) {
    fail("wrapper-py/pyproject.toml is missing a wheel packages entry");
  }
  return match[1];
}

function assertAllEqual(entries, message) {
  const values = new Set(entries.map(([, value]) => value));
  if (values.size !== 1) {
    fail(
      `${message}: ${entries
        .map(([name, value]) => `${name}=${value}`)
        .join(", ")}`
    );
  }
}

function assertDeepEqual(actual, expected, message) {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    fail(`${message}\nactual=${JSON.stringify(actual)}\nexpected=${JSON.stringify(expected)}`);
  }
}

function fail(message) {
  throw new Error(message);
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function isNotGitRepositoryError(error) {
  const stderr = typeof error?.stderr === "string" ? error.stderr : "";
  return error?.status === 128 && /not a git repository/i.test(stderr);
}
