import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

test("renamed pattern1 bootstrap is buildable and wrapper-executable without manual identity repair", () => {
  const tempRoot = mkdtempSync(path.join(os.tmpdir(), "ossplate-agentcode-bootstrap-"));
  const targetRoot = path.join(tempRoot, "agentcode");
  const python = findPython();

  try {
    execFileSync(
      "cargo",
      [
        "run",
        "--manifest-path",
        path.join(repoRoot, "core-rs", "Cargo.toml"),
        "--",
        "create",
        targetRoot,
        "--name",
        "agentcode",
        "--command",
        "agentcode",
        "--description",
        "Build and ship the agentcode CLI through Rust, npm, and PyPI.",
        "--repository",
        "https://github.com/stefdevscore/agentcode",
        "--author-name",
        "Azk",
        "--author-email",
        "azk@example.com",
        "--license",
        "Apache-2.0"
      ],
      { cwd: repoRoot, stdio: "ignore" }
    );

    execFileSync("cargo", ["run", "--manifest-path", "core-rs/Cargo.toml", "--", "validate", "--json"], {
      cwd: targetRoot,
      stdio: "ignore"
    });
    execFileSync("cargo", ["run", "--manifest-path", "core-rs/Cargo.toml", "--", "sync", "--check"], {
      cwd: targetRoot,
      stdio: "ignore"
    });

    injectPingCommand(targetRoot);

    execFileSync("cargo", ["build"], {
      cwd: path.join(targetRoot, "core-rs"),
      stdio: "ignore"
    });
    execFileSync("node", ["scripts/stage-distribution-assets.mjs"], {
      cwd: targetRoot,
      stdio: "ignore"
    });
    execFileSync("npm", ["install"], {
      cwd: path.join(targetRoot, "wrapper-js"),
      stdio: "ignore"
    });
    execFileSync("npm", ["run", "build"], {
      cwd: path.join(targetRoot, "wrapper-js"),
      stdio: "ignore"
    });
    execFileSync("node", ["scripts/stage-distribution-assets.mjs"], {
      cwd: targetRoot,
      stdio: "ignore"
    });

    const binary = path.join(
      targetRoot,
      "core-rs",
      "target",
      "debug",
      process.platform === "win32" ? "agentcode.exe" : "agentcode"
    );
    assert.ok(existsSync(binary), `expected built binary at ${binary}`);
    assert.equal(
      execFileSync(binary, ["version"], { cwd: targetRoot, encoding: "utf8" }).trim(),
      '{"tool":"agentcode","version":"0.2.3"}'
    );
    assert.equal(
      execFileSync(binary, ["ping"], { cwd: targetRoot, encoding: "utf8" }).trim(),
      "pong"
    );

    const jsTool = path.join(targetRoot, "wrapper-js", "bin", "agentcode.js");
    assert.ok(existsSync(jsTool), `expected JS wrapper launcher at ${jsTool}`);
    const wrapperEnv = { ...process.env, OSSPLATE_BINARY: binary };
    assert.equal(
      execFileSync("node", [jsTool, "version"], {
        cwd: path.join(targetRoot, "wrapper-js"),
        encoding: "utf8",
        env: wrapperEnv
      }).trim(),
      '{"tool":"agentcode","version":"0.2.3"}'
    );
    assert.equal(
      execFileSync("node", [jsTool, "ping"], {
        cwd: path.join(targetRoot, "wrapper-js"),
        encoding: "utf8",
        env: wrapperEnv
      }).trim(),
      "pong"
    );

    const pyEnv = {
      ...process.env,
      OSSPLATE_BINARY: binary,
      PYTHONPATH: path.join(targetRoot, "wrapper-py", "src")
    };
    assert.equal(
      execFileSync(python, ["-m", "agentcode.cli", "version"], {
        cwd: path.join(targetRoot, "wrapper-py"),
        encoding: "utf8",
        env: pyEnv
      }).trim(),
      '{"tool":"agentcode","version":"0.2.3"}'
    );
    assert.equal(
      execFileSync(python, ["-m", "agentcode.cli", "ping"], {
        cwd: path.join(targetRoot, "wrapper-py"),
        encoding: "utf8",
        env: pyEnv
      }).trim(),
      "pong"
    );

    execFileSync("node", ["scripts/release-check.mjs", "release-state"], {
      cwd: targetRoot,
      stdio: "ignore"
    });
  } finally {
    rmSync(tempRoot, { recursive: true, force: true });
  }
});

function injectPingCommand(targetRoot) {
  const mainPath = path.join(targetRoot, "core-rs", "src", "main.rs");
  const original = readFileSync(mainPath, "utf8");
  const updated = original
    .replace("    /// Print tool version information\n    Version,\n", "    /// Print tool version information\n    Version,\n    /// Minimal spike command\n    Ping,\n")
    .replace(
      "        Commands::Version => {\n            println!(\"{}\", render_version_output()?);\n            Ok(())\n        }\n",
      "        Commands::Version => {\n            println!(\"{}\", render_version_output()?);\n            Ok(())\n        }\n        Commands::Ping => {\n            println!(\"pong\");\n            Ok(())\n        }\n"
    )
    .replace('#[command(name = "ossplate")]', '#[command(name = "agentcode")]')
    .replace(
      '    about = "Validate and sync a multi-registry OSS scaffold"\n',
      '    about = "Build and ship the agentcode CLI through Rust, npm, and PyPI."\n'
    )
    .replace('        eprintln!("ossplate: {error}");', '        eprintln!("agentcode: {error}");');
  writeFileSync(mainPath, updated);
}

function findPython() {
  for (const candidate of [
    "python3.14",
    "python3.13",
    "python3.12",
    "python3.11",
    "python3.10",
    "python3"
  ]) {
    try {
      execFileSync(candidate, ["-c", "import sys; raise SystemExit(0 if sys.version_info >= (3, 10) else 1)"], {
        stdio: "ignore"
      });
      return candidate;
    } catch {}
  }
  throw new Error("no Python 3.10+ interpreter found");
}
