import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import {
  GENERATED_AUTHOR_EMAIL_PLACEHOLDER,
  GENERATED_AUTHOR_NAME_PLACEHOLDER,
  assertProjectMetadataReady,
  readScaffoldPayload,
  readTomlSectionValue,
  repoRoot
} from "./release-state.mjs";
import { getRuntimeTargets, runtimePackageName } from "./runtime-targets.mjs";

const TEMPLATE_ONLY_PATHS = readScaffoldPayload().templateOnlyPaths ?? [];
const GENERATED_DOC_PATHS = [
  "README.md",
  "docs/README.md",
  "docs/releases.md",
  "wrapper-js/README.md",
  "wrapper-py/README.md"
];
const FORBIDDEN_PUBLIC_PHRASES = ["ossplate create", "stefdevscore/ossplate", "Adoption Guide"];

const PRIMARY_OVERRIDES = {
  name: "Dogfood Control Plane",
  description: "Ship the dogfood-control CLI across Cargo, npm, and PyPI.",
  repository: "https://github.com/acme/dogfood-control",
  license: "Apache-2.0",
  authorName: "Acme OSS",
  authorEmail: "oss@acme.dev",
  rustCrate: "dogfood-control",
  npmPackage: "@acme/dogfood-control",
  pythonPackage: "dogfood_control",
  command: "dogfood-control"
};

const PLACEHOLDER_OVERRIDES = {
  name: "Placeholder Probe",
  rustCrate: "placeholder-probe",
  npmPackage: "@acme/placeholder-probe",
  pythonPackage: "placeholder_probe",
  command: "placeholder-probe"
};

export function runGeneratedProjectDogfood(root = repoRoot) {
  const tempRoot = mkdtempSync(join(tmpdir(), "ossplate-generated-dogfood-"));

  try {
    const primaryRoot = join(tempRoot, "primary-project");
    createProject(root, primaryRoot, PRIMARY_OVERRIDES);
    assertGeneratedProject(primaryRoot, PRIMARY_OVERRIDES);
    assertRebootstrap(root, primaryRoot, PRIMARY_OVERRIDES, tempRoot);

    const placeholderRoot = join(tempRoot, "placeholder-project");
    createProject(root, placeholderRoot, PLACEHOLDER_OVERRIDES);
    assertPlaceholderPolicy(placeholderRoot);
  } finally {
    rmSync(tempRoot, { recursive: true, force: true });
  }
}

function createProject(root, targetRoot, overrides) {
  execFileSync(
    "cargo",
    [
      "run",
      "--quiet",
      "--manifest-path",
      join(root, "core-rs", "Cargo.toml"),
      "--",
      "create",
      targetRoot,
      "--json",
      ...buildOverrideArgs(overrides)
    ],
    {
      cwd: root,
      encoding: "utf8",
      stdio: "pipe"
    }
  );
}

function assertGeneratedProject(projectRoot, expected) {
  const validate = execToolJson(projectRoot, "validate", "--path", projectRoot, "--json");
  assert.equal(validate.ok, true, "generated project must validate successfully");
  assert.deepEqual(validate.issues, [], "generated project must not report validation issues");
  assert.deepEqual(validate.warnings, [], "generated project with explicit metadata should not warn");

  execTool(projectRoot, "sync", "--path", projectRoot, "--check", "--json");

  const inspect = execToolJson(projectRoot, "inspect", "--path", projectRoot, "--json");
  assert.equal(inspect.config.template.is_canonical, false, "generated project must not be canonical");
  assert.equal(inspect.config.project.name, expected.name);
  assert.equal(inspect.config.project.description, expected.description);
  assert.equal(inspect.config.project.repository, expected.repository);
  assert.equal(inspect.config.project.license, expected.license);
  assert.equal(inspect.config.author.name, expected.authorName);
  assert.equal(inspect.config.author.email, expected.authorEmail);
  assert.equal(inspect.config.packages.rust_crate, expected.rustCrate);
  assert.equal(inspect.config.packages.npm_package, expected.npmPackage);
  assert.equal(inspect.config.packages.python_package, expected.pythonPackage);
  assert.equal(inspect.config.packages.command, expected.command);

  const configText = readProjectText(projectRoot, "ossplate.toml");
  assert.equal(readTomlBoolSectionValue(configText, "template", "is_canonical", "ossplate.toml"), false);
  assert.equal(readTomlSectionValue(configText, "packages", "command", "ossplate.toml"), expected.command);
  assert.equal(readTomlSectionValue(configText, "packages", "rust_crate", "ossplate.toml"), expected.rustCrate);
  assert.equal(readTomlSectionValue(configText, "packages", "npm_package", "ossplate.toml"), expected.npmPackage);
  assert.equal(readTomlSectionValue(configText, "packages", "python_package", "ossplate.toml"), expected.pythonPackage);
  assert.notEqual(
    readTomlSectionValue(configText, "author", "name", "ossplate.toml"),
    "Stef",
    "generated project must not inherit template maintainer name"
  );
  assert.notEqual(
    readTomlSectionValue(configText, "author", "email", "ossplate.toml"),
    "stefdevscore@github.com",
    "generated project must not inherit template maintainer email"
  );

  assertGeneratedDocs(projectRoot);
  assertTemplateOnlyPathsFiltered(projectRoot);
  assertWrapperMetadata(projectRoot, expected);
  assertEmbeddedPayload(projectRoot, expected);
  assertPublicRuntimeNames(inspect, expected.npmPackage);
  assertProjectMetadataReady(
    {
      projectDescription: expected.description,
      projectRepository: expected.repository,
      authorName: expected.authorName,
      authorEmail: expected.authorEmail,
      command: expected.command
    },
    { name: expected.npmPackage }
  );
}

function assertGeneratedDocs(projectRoot) {
  for (const relativePath of GENERATED_DOC_PATHS) {
    const content = readProjectText(projectRoot, relativePath);
    for (const phrase of FORBIDDEN_PUBLIC_PHRASES) {
      assert.ok(
        !content.includes(phrase),
        `${relativePath} must not contain template-only phrase ${JSON.stringify(phrase)}`
      );
    }
  }
}

function assertTemplateOnlyPathsFiltered(projectRoot) {
  for (const relativePath of TEMPLATE_ONLY_PATHS) {
    assert.ok(
      !existsSync(join(projectRoot, relativePath)),
      `generated project must not include template-only path ${relativePath}`
    );
    assert.ok(
      !existsSync(join(projectRoot, "core-rs", "embedded-template-root", relativePath)),
      `embedded payload must not include template-only path ${relativePath}`
    );
  }
}

function assertWrapperMetadata(projectRoot, expected) {
  const jsPackage = readProjectJson(projectRoot, "wrapper-js/package.json");
  assert.equal(jsPackage.name, expected.npmPackage);
  assert.deepEqual(Object.keys(jsPackage.bin ?? {}), [expected.command]);
  assert.equal(jsPackage.bin[expected.command], `bin/${expected.command}.js`);
  assert.equal(jsPackage.repository?.url, expected.repository);

  const jsLock = readProjectJson(projectRoot, "wrapper-js/package-lock.json");
  assert.equal(jsLock.packages?.[""]?.name, expected.npmPackage);
  for (const entry of getRuntimeTargets()) {
    const packageName = runtimePackageName(expected.npmPackage, entry.target);
    assert.equal(
      jsPackage.optionalDependencies?.[packageName],
      jsPackage.version,
      `wrapper-js/package.json must declare runtime package ${packageName}`
    );
    assert.equal(
      jsLock.packages?.[`node_modules/${packageName}`]?.version,
      jsPackage.version,
      `wrapper-js/package-lock.json must include runtime package ${packageName}`
    );
  }
  assert.ok(
    !JSON.stringify(jsLock).includes("ossplate-darwin-arm64"),
    "wrapper-js/package-lock.json must not reference canonical runtime package names"
  );

  const pyproject = readProjectText(projectRoot, "wrapper-py/pyproject.toml");
  assert.equal(readTomlSectionValue(pyproject, "project", "name", "wrapper-py/pyproject.toml"), expected.pythonPackage);
  assert.ok(
    pyproject.includes(`packages = ["src/${expected.pythonPackage}"]`),
    "wrapper-py/pyproject.toml must point wheel packages at the generated python package directory"
  );

  const cargoToml = readProjectText(projectRoot, "core-rs/Cargo.toml");
  assert.equal(readTomlSectionValue(cargoToml, "package", "name", "core-rs/Cargo.toml"), expected.rustCrate);
  assert.ok(
    cargoToml.includes(`name = "${expected.command}"`),
    "generated Rust binary name must match the configured command"
  );
}

function assertEmbeddedPayload(projectRoot, expected) {
  const embeddedRoot = join(projectRoot, "core-rs", "embedded-template-root");
  const embeddedConfig = readProjectText(projectRoot, "core-rs/embedded-template-root/ossplate.toml");
  assert.equal(
    readTomlBoolSectionValue(
      embeddedConfig,
      "template",
      "is_canonical",
      "core-rs/embedded-template-root/ossplate.toml"
    ),
    false
  );
  assert.equal(
    readTomlSectionValue(
      embeddedConfig,
      "packages",
      "npm_package",
      "core-rs/embedded-template-root/ossplate.toml"
    ),
    expected.npmPackage
  );
  assert.equal(
    readTomlSectionValue(
      embeddedConfig,
      "packages",
      "python_package",
      "core-rs/embedded-template-root/ossplate.toml"
    ),
    expected.pythonPackage
  );

  const embeddedJsPackage = readProjectJson(projectRoot, "core-rs/embedded-template-root/wrapper-js/package.json");
  assert.equal(embeddedJsPackage.name, expected.npmPackage);
  const embeddedCargoToml = readProjectText(projectRoot, "core-rs/embedded-template-root/core-rs/Cargo.toml");
  assert.equal(
    readTomlSectionValue(
      embeddedCargoToml,
      "package",
      "name",
      "core-rs/embedded-template-root/core-rs/Cargo.toml"
    ),
    expected.rustCrate
  );
  const embeddedPyproject = readProjectText(projectRoot, "core-rs/embedded-template-root/wrapper-py/pyproject.toml");
  assert.equal(
    readTomlSectionValue(
      embeddedPyproject,
      "project",
      "name",
      "core-rs/embedded-template-root/wrapper-py/pyproject.toml"
    ),
    expected.pythonPackage
  );

  for (const relativePath of GENERATED_DOC_PATHS) {
    const embeddedPath = join(embeddedRoot, relativePath);
    if (!existsSync(embeddedPath)) {
      continue;
    }
    const content = readFileSync(embeddedPath, "utf8");
    for (const phrase of FORBIDDEN_PUBLIC_PHRASES) {
      assert.ok(
        !content.includes(phrase),
        `embedded ${relativePath} must not contain template-only phrase ${JSON.stringify(phrase)}`
      );
    }
  }
}

function assertPublicRuntimeNames(inspect, npmPackage) {
  const derivedPackages = inspect.derived?.runtimePackages ?? [];
  const expected = new Set(getRuntimeTargets().map((entry) => runtimePackageName(npmPackage, entry.target)));
  assert.equal(derivedPackages.length, expected.size);
  for (const runtimePackage of derivedPackages) {
    assert.ok(
      expected.has(runtimePackage.packageName),
      `inspect runtime package ${runtimePackage.packageName} must match the generated npm identity`
    );
  }
}

function assertRebootstrap(root, primaryRoot, expected, tempRoot) {
  execFileSync("cargo", ["build", "--manifest-path", "core-rs/Cargo.toml"], {
    cwd: primaryRoot,
    stdio: "pipe"
  });
  const binaryPath = resolveGeneratedBinary(primaryRoot, expected);
  const rebootstrapRoot = join(tempRoot, "rebootstrap-project");
  execFileSync(binaryPath, ["create", rebootstrapRoot, "--json", ...buildOverrideArgs(expected)], {
    cwd: tempRoot,
    encoding: "utf8",
    stdio: "pipe"
  });
  assertGeneratedProject(rebootstrapRoot, expected);

  const rebootstrapInspect = execToolJson(rebootstrapRoot, "inspect", "--path", rebootstrapRoot, "--json");
  assert.equal(rebootstrapInspect.config.template.is_canonical, false);
}

function assertPlaceholderPolicy(projectRoot) {
  const validate = execToolJson(projectRoot, "validate", "--path", projectRoot, "--json");
  assert.equal(validate.ok, true, "placeholder generated project must still validate locally");
  assert.ok(validate.warnings.length >= 4, "placeholder generated project must emit metadata warnings");

  const configText = readProjectText(projectRoot, "ossplate.toml");
  assert.equal(
    readTomlSectionValue(configText, "project", "repository", "ossplate.toml"),
    "https://example.com/replace-with-your-repository"
  );
  assert.equal(
    readTomlSectionValue(configText, "author", "name", "ossplate.toml"),
    GENERATED_AUTHOR_NAME_PLACEHOLDER
  );
  assert.equal(
    readTomlSectionValue(configText, "author", "email", "ossplate.toml"),
    GENERATED_AUTHOR_EMAIL_PLACEHOLDER
  );

  assert.throws(
    () =>
      execFileSync("node", ["scripts/release-check.mjs", "publish-readiness", "publish"], {
        cwd: projectRoot,
        encoding: "utf8",
        stdio: "pipe"
      }),
    /publish readiness requires real project metadata/
  );
}

function execToolJson(projectRoot, ...args) {
  return JSON.parse(execTool(projectRoot, ...args));
}

function execTool(projectRoot, ...args) {
  return execFileSync("cargo", ["run", "--quiet", "--manifest-path", "core-rs/Cargo.toml", "--", ...args], {
    cwd: projectRoot,
    encoding: "utf8",
    stdio: "pipe"
  });
}

function resolveGeneratedBinary(projectRoot, expected) {
  const suffix = process.platform === "win32" ? ".exe" : "";
  const candidates = [expected.command, expected.rustCrate].map((name) =>
    join(projectRoot, "core-rs", "target", "debug", `${name}${suffix}`)
  );
  const match = candidates.find((candidate) => existsSync(candidate));
  if (!match) {
    throw new Error(`failed to locate generated project binary; tried ${candidates.join(", ")}`);
  }
  return match;
}

function buildOverrideArgs(overrides) {
  const args = [];
  for (const [flag, value] of [
    ["--name", overrides.name],
    ["--description", overrides.description],
    ["--repository", overrides.repository],
    ["--license", overrides.license],
    ["--author-name", overrides.authorName],
    ["--author-email", overrides.authorEmail],
    ["--rust-crate", overrides.rustCrate],
    ["--npm-package", overrides.npmPackage],
    ["--python-package", overrides.pythonPackage],
    ["--command", overrides.command]
  ]) {
    if (value) {
      args.push(flag, value);
    }
  }
  return args;
}

function readProjectJson(projectRoot, relativePath) {
  return JSON.parse(readProjectText(projectRoot, relativePath));
}

function readProjectText(projectRoot, relativePath) {
  return readFileSync(join(projectRoot, relativePath), "utf8");
}

function readTomlBoolSectionValue(content, sectionName, key, fileLabel) {
  const sectionPattern = new RegExp(`^\\[${escapeRegExp(sectionName)}\\]\\s*$`, "m");
  const sectionMatch = sectionPattern.exec(content);
  if (!sectionMatch) {
    throw new Error(`failed to read ${key} from ${fileLabel}`);
  }

  const sectionStart = sectionMatch.index + sectionMatch[0].length;
  const nextSectionOffset = content.slice(sectionStart).search(/^\[[^\]]+\]\s*$/m);
  const sectionBody =
    nextSectionOffset === -1
      ? content.slice(sectionStart)
      : content.slice(sectionStart, sectionStart + nextSectionOffset);
  const valuePattern = new RegExp(`^${escapeRegExp(key)}\\s*=\\s*(true|false)\\s*$`, "m");
  const valueMatch = valuePattern.exec(sectionBody);
  if (!valueMatch) {
    throw new Error(`failed to read ${key} from ${fileLabel}`);
  }
  return valueMatch[1] === "true";
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

if (import.meta.url === `file://${process.argv[1]}`) {
  runGeneratedProjectDogfood();
  console.log("generated project dogfood ok");
}
