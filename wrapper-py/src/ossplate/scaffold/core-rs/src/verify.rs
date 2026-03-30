use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct VerifyStepResult {
    pub(crate) name: String,
    pub(crate) ok: bool,
    #[serde(rename = "exitCode")]
    pub(crate) exit_code: i32,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) skipped: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reason: Option<String>,
}

#[derive(Debug, Clone)]
struct VerifyStep {
    name: &'static str,
    cwd: &'static str,
    cmd: Vec<String>,
    env: BTreeMap<String, String>,
    skip: Option<SkipRule>,
}

#[derive(Debug, Clone)]
struct SkipRule {
    reason: String,
}

trait VerifyRunner {
    fn run(&self, root: &Path, cmd: &[String]) -> Result<VerifyStepResult>;
}

struct CommandVerifyRunner;

impl VerifyRunner for CommandVerifyRunner {
    fn run(&self, root: &Path, cmd: &[String]) -> Result<VerifyStepResult> {
        run_command(root, ".", &BTreeMap::new(), cmd)
    }
}

fn run_command(
    root: &Path,
    cwd: &str,
    env: &BTreeMap<String, String>,
    cmd: &[String],
) -> Result<VerifyStepResult> {
    let program = cmd
        .first()
        .cloned()
        .context("verify step requires a program")?;
    let output = Command::new(&program)
        .args(&cmd[1..])
        .current_dir(root.join(cwd))
        .envs(env)
        .output()
        .with_context(|| format!("failed to start verify step: {program}"))?;
    Ok(VerifyStepResult {
        name: String::new(),
        ok: output.status.success(),
        exit_code: output.status.code().unwrap_or(1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        skipped: false,
        reason: None,
    })
}

pub(crate) fn verify_repo_output(root: &Path) -> Result<crate::output::VerifyOutput> {
    let steps = verify_repo_steps(root, &CommandVerifyRunner)?;
    Ok(crate::output::VerifyOutput {
        ok: steps.iter().all(|step| step.ok || step.skipped),
        steps,
    })
}

fn verify_repo_steps(root: &Path, runner: &dyn VerifyRunner) -> Result<Vec<VerifyStepResult>> {
    let (package_name, version) = wrapper_js_identity(root)?;
    let js_installable = npm_version_exists(&format!("{package_name}@{version}"));
    let js_lockfile_mode = if js_installable {
        "resolved"
    } else {
        "placeholder"
    };

    let steps = verify_steps(root, js_lockfile_mode, js_installable)?;
    run_verify_steps(root, &steps, runner)
}

fn verify_steps(
    root: &Path,
    js_lockfile_mode: &str,
    js_installable: bool,
) -> Result<Vec<VerifyStep>> {
    let python = find_python(root)?;
    let mut steps = vec![
        step(
            "rust:fmt",
            ".",
            vec![
                "cargo",
                "fmt",
                "--check",
                "--manifest-path",
                "core-rs/Cargo.toml",
            ],
        ),
        step(
            "rust:clippy",
            ".",
            vec![
                "cargo",
                "clippy",
                "--manifest-path",
                "core-rs/Cargo.toml",
                "--",
                "-D",
                "warnings",
            ],
        ),
        step(
            "rust:test",
            ".",
            vec!["cargo", "test", "--manifest-path", "core-rs/Cargo.toml"],
        ),
        step(
            "tool:validate",
            ".",
            vec![
                "cargo",
                "run",
                "--quiet",
                "--manifest-path",
                "core-rs/Cargo.toml",
                "--",
                "validate",
                "--path",
                &root.display().to_string(),
                "--json",
            ],
        ),
        step(
            "tool:sync-check",
            ".",
            vec![
                "cargo",
                "run",
                "--quiet",
                "--manifest-path",
                "core-rs/Cargo.toml",
                "--",
                "sync",
                "--path",
                &root.display().to_string(),
                "--check",
                "--json",
            ],
        ),
        step(
            "release:plan-test",
            ".",
            vec!["node", "--test", "scripts/release-plan.test.mjs"],
        ),
        step(
            "release:check-test",
            ".",
            vec!["node", "--test", "scripts/release-check.test.mjs"],
        ),
        step(
            "release:state-test",
            ".",
            vec!["node", "--test", "scripts/release-state.test.mjs"],
        ),
        step(
            "bootstrap:pattern1-test",
            ".",
            vec!["node", "--test", "scripts/bootstrap-pattern1.test.mjs"],
        ),
        step(
            "publish:local-test",
            ".",
            vec!["node", "--test", "scripts/publish-local.test.mjs"],
        ),
        step(
            "scaffold:mirrors-assert",
            ".",
            vec!["node", "scripts/release-check.mjs", "scaffold-mirrors"],
        ),
        step(
            "release:assert",
            ".",
            vec!["node", "scripts/release-check.mjs", "release-state"],
        ),
        step(
            "js:lockfile-assert",
            ".",
            vec![
                "node",
                "scripts/assert-js-lockfile-state.mjs",
                js_lockfile_mode,
            ],
        ),
        step(
            "publish:assert",
            ".",
            vec![
                "node",
                "scripts/release-check.mjs",
                "publish-readiness",
                "publish",
            ],
        ),
    ];

    if js_installable {
        steps.push(step("js:test", "wrapper-js", vec!["npm", "test"]));
        steps.push(step(
            "js:pack",
            "wrapper-js",
            vec!["npm", "pack", "--dry-run"],
        ));
    } else {
        let reason = format!(
            "current npm version {} is not published yet; placeholder lockfile state is expected",
            wrapper_js_identity(root)?.1
        );
        steps.push(VerifyStep {
            name: "js:test",
            cwd: "wrapper-js",
            cmd: Vec::new(),
            env: BTreeMap::new(),
            skip: Some(SkipRule {
                reason: reason.clone(),
            }),
        });
        steps.push(VerifyStep {
            name: "js:pack",
            cwd: "wrapper-js",
            cmd: Vec::new(),
            env: BTreeMap::new(),
            skip: Some(SkipRule { reason }),
        });
    }

    let mut py_env = BTreeMap::new();
    py_env.insert("PYTHONPATH".to_string(), "src".to_string());
    steps.push(VerifyStep {
        name: "py:test",
        cwd: "wrapper-py",
        cmd: vec![
            python,
            "-m".to_string(),
            "unittest".to_string(),
            "discover".to_string(),
            "-s".to_string(),
            "tests".to_string(),
            "-p".to_string(),
            "test_*.py".to_string(),
        ],
        env: py_env,
        skip: None,
    });

    Ok(steps)
}

fn run_verify_steps(
    root: &Path,
    steps: &[VerifyStep],
    runner: &dyn VerifyRunner,
) -> Result<Vec<VerifyStepResult>> {
    let mut results = Vec::new();
    for step in steps {
        if let Some(skip) = &step.skip {
            results.push(VerifyStepResult {
                name: step.name.to_string(),
                ok: true,
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
                skipped: true,
                reason: Some(skip.reason.clone()),
            });
            continue;
        }

        let mut result = run_command(root, step.cwd, &step.env, &step.cmd)
            .or_else(|_| runner.run(root, &step.cmd))?;
        result.name = step.name.to_string();
        if !result.ok {
            results.push(result);
            break;
        }
        results.push(result);
    }
    Ok(results)
}

fn step(name: &'static str, cwd: &'static str, parts: Vec<&str>) -> VerifyStep {
    VerifyStep {
        name,
        cwd,
        cmd: parts.into_iter().map(str::to_string).collect(),
        env: BTreeMap::new(),
        skip: None,
    }
}

fn wrapper_js_identity(root: &Path) -> Result<(String, String)> {
    let package = root.join("wrapper-js/package.json");
    let contents = std::fs::read_to_string(&package)
        .with_context(|| format!("failed to read {}", package.display()))?;
    let value: serde_json::Value = serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse {}", package.display()))?;
    let name = value["name"]
        .as_str()
        .map(ToString::to_string)
        .context("wrapper-js/package.json is missing name")?;
    let version = value["version"]
        .as_str()
        .map(ToString::to_string)
        .context("wrapper-js/package.json is missing version")?;
    Ok((name, version))
}

fn npm_version_exists(spec: &str) -> bool {
    Command::new("npm")
        .args(["view", spec, "version"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn find_python(root: &Path) -> Result<String> {
    let candidates = [
        "python3.14",
        "python3.13",
        "python3.12",
        "python3.11",
        "python3.10",
        "python3",
    ];
    for candidate in candidates {
        let status = Command::new(candidate)
            .args([
                "-c",
                "import sys; raise SystemExit(0 if sys.version_info >= (3, 10) else 1)",
            ])
            .current_dir(root)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        if matches!(status, Ok(value) if value.success()) {
            return Ok(candidate.to_string());
        }
    }
    bail!("verify requires Python 3.10+ on PATH")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    struct StubRunner {
        results: Arc<Mutex<VecDeque<VerifyStepResult>>>,
    }

    impl StubRunner {
        fn new(results: Vec<VerifyStepResult>) -> Self {
            Self {
                results: Arc::new(Mutex::new(results.into())),
            }
        }
    }

    impl VerifyRunner for StubRunner {
        fn run(&self, _root: &Path, _cmd: &[String]) -> Result<VerifyStepResult> {
            self.results
                .lock()
                .unwrap()
                .pop_front()
                .context("missing stub verify result")
        }
    }

    #[test]
    fn render_verify_output_marks_skipped_steps_as_non_blocking() {
        let json = crate::output::render_verify_output(vec![
            VerifyStepResult {
                name: "a".into(),
                ok: true,
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
                skipped: false,
                reason: None,
            },
            VerifyStepResult {
                name: "b".into(),
                ok: true,
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
                skipped: true,
                reason: Some("skip".into()),
            },
        ])
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["ok"], true);
        assert_eq!(value["steps"][1]["skipped"], true);
    }

    #[test]
    fn verify_repo_steps_stops_on_first_failure() {
        let root = std::env::temp_dir();
        let runner = StubRunner::new(vec![VerifyStepResult {
            name: String::new(),
            ok: false,
            exit_code: 1,
            stdout: String::new(),
            stderr: "boom".into(),
            skipped: false,
            reason: None,
        }]);
        let steps = vec![VerifyStep {
            name: "broken",
            cwd: ".",
            cmd: vec!["false".into()],
            env: BTreeMap::new(),
            skip: None,
        }];
        let results = run_verify_steps(&root, &steps, &runner).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].ok, false);
        assert_eq!(results[0].name, "broken");
    }

    #[test]
    fn verify_repo_steps_preserves_skip_results() {
        let root = std::env::temp_dir();
        let runner = StubRunner::new(vec![VerifyStepResult {
            name: String::new(),
            ok: true,
            exit_code: 0,
            stdout: "ok".into(),
            stderr: String::new(),
            skipped: false,
            reason: None,
        }]);
        let steps = vec![
            VerifyStep {
                name: "skip",
                cwd: ".",
                cmd: Vec::new(),
                env: BTreeMap::new(),
                skip: Some(SkipRule {
                    reason: "skip reason".into(),
                }),
            },
            VerifyStep {
                name: "run",
                cwd: ".",
                cmd: vec!["true".into()],
                env: BTreeMap::new(),
                skip: None,
            },
        ];
        let results = run_verify_steps(&root, &steps, &runner).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].skipped, true);
        assert_eq!(results[0].reason.as_deref(), Some("skip reason"));
        assert_eq!(results[1].name, "run");
    }
}
