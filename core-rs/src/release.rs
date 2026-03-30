use crate::source_checkout::ensure_source_checkout;
use anyhow::{bail, Context, Result};
use clap::ValueEnum;
use serde::Serialize;
use serde_json::json;
use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, serde::Serialize)]
pub(crate) enum PublishRegistry {
    All,
    Npm,
    Pypi,
    Cargo,
}

pub(crate) fn publish_repo(
    root: &Path,
    dry_run: bool,
    registry: PublishRegistry,
    skip_existing: bool,
) -> Result<()> {
    publish_repo_with_runner(
        root,
        dry_run,
        registry,
        skip_existing,
        &NodePublishHelperRunner,
    )
}

fn publish_repo_with_runner(
    root: &Path,
    dry_run: bool,
    registry: PublishRegistry,
    skip_existing: bool,
    runner: &dyn PublishHelperRunner,
) -> Result<()> {
    ensure_source_checkout(root, "publish requires")?;
    let invocation = plan_publish_helper_invocation(root, dry_run, registry, skip_existing)?;
    let status = runner.run(&invocation.root, &invocation.args)?;
    if status.success() {
        Ok(())
    } else {
        bail!("publish failed")
    }
}

struct PublishHelperInvocation {
    root: std::path::PathBuf,
    helper: std::path::PathBuf,
    registry: PublishRegistry,
    dry_run: bool,
    skip_existing: bool,
    args: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct PublishHostInfo {
    target: String,
    os: String,
    arch: String,
}

fn plan_publish_helper_invocation(
    root: &Path,
    dry_run: bool,
    registry: PublishRegistry,
    skip_existing: bool,
) -> Result<PublishHelperInvocation> {
    let root = root
        .canonicalize()
        .with_context(|| format!("failed to canonicalize publish path {}", root.display()))?;
    let script_path = publish_helper_script_path(&root)?;
    let args = build_publish_args(&script_path, &root, dry_run, registry, skip_existing);
    Ok(PublishHelperInvocation {
        root,
        helper: script_path,
        registry,
        dry_run,
        skip_existing,
        args,
    })
}

fn publish_helper_script_path(root: &Path) -> Result<std::path::PathBuf> {
    let script_path = root.join("scripts/publish-local.mjs");
    if !script_path.is_file() {
        bail!(
            "publish requires a full scaffold source checkout; missing {}",
            script_path.display()
        );
    }
    Ok(script_path)
}

fn build_publish_args(
    script_path: &Path,
    root: &Path,
    dry_run: bool,
    registry: PublishRegistry,
    skip_existing: bool,
) -> Vec<String> {
    let mut args = vec![
        script_path.to_string_lossy().to_string(),
        "--root".to_string(),
        root.to_string_lossy().to_string(),
        "--registry".to_string(),
        registry_arg(registry).to_string(),
    ];
    if dry_run {
        args.push("--dry-run".to_string());
    }
    if skip_existing {
        args.push("--skip-existing".to_string());
    }
    args
}

fn registry_arg(registry: PublishRegistry) -> &'static str {
    match registry {
        PublishRegistry::All => "all",
        PublishRegistry::Npm => "npm",
        PublishRegistry::Pypi => "pypi",
        PublishRegistry::Cargo => "cargo",
    }
}

trait PublishHelperRunner {
    fn run(&self, root: &Path, args: &[String]) -> Result<ExitStatus>;
}

struct NodePublishHelperRunner;

impl PublishHelperRunner for NodePublishHelperRunner {
    fn run(&self, root: &Path, args: &[String]) -> Result<ExitStatus> {
        Command::new("node")
            .args(args)
            .current_dir(root)
            .status()
            .context("failed to start local publish helper via node")
    }
}

pub(crate) fn render_publish_plan(
    root: &Path,
    dry_run: bool,
    registry: PublishRegistry,
    skip_existing: bool,
) -> Result<String> {
    ensure_source_checkout(root, "publish requires")?;
    let invocation = plan_publish_helper_invocation(root, dry_run, registry, skip_existing)?;
    let selected_registries = selected_registries(registry)
        .into_iter()
        .map(|entry| registry_arg(entry).to_string())
        .collect::<Vec<_>>();
    let host = resolve_host_target();
    let preflight = build_publish_preflight(&selected_registries, dry_run, skip_existing)?;
    crate::output::render_publish_plan_output(crate::output::PublishPlanOutput {
        ok: true,
        root: invocation.root.display().to_string(),
        registry: invocation.registry,
        dry_run: invocation.dry_run,
        skip_existing: invocation.skip_existing,
        helper: invocation.helper.display().to_string(),
        argv: invocation.args,
        selected_registries,
        host: serde_json::to_value(host)?,
        preflight,
    })
}

fn selected_registries(registry: PublishRegistry) -> Vec<PublishRegistry> {
    match registry {
        PublishRegistry::All => vec![
            PublishRegistry::Npm,
            PublishRegistry::Pypi,
            PublishRegistry::Cargo,
        ],
        other => vec![other],
    }
}

fn build_publish_preflight(
    selected_registries: &[String],
    dry_run: bool,
    skip_existing: bool,
) -> Result<serde_json::Value> {
    let mut tools = Vec::new();
    let mut auth = Vec::new();
    let mut issues = Vec::new();
    let selected = selected_registries
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    let node_required = true;
    let cargo_required = selected
        .iter()
        .any(|entry| matches!(*entry, "npm" | "pypi" | "cargo"));
    let npm_required = selected.contains(&"npm");
    let curl_required = selected.contains(&"cargo") && skip_existing;
    let python_required = selected.contains(&"pypi");
    let npm_publish_required = selected.contains(&"npm") && !dry_run;
    let cargo_publish_required = selected.contains(&"cargo") && !dry_run && !skip_existing;
    let pypi_publish_required = selected.contains(&"pypi") && !dry_run;

    for (name, required_now, required_publish) in [
        ("node", node_required, node_required),
        ("cargo", cargo_required, cargo_required),
        ("npm", npm_required && !dry_run, npm_required),
        ("curl", curl_required && !dry_run, curl_required),
    ] {
        let available = !required_publish || command_exists(name);
        if required_now && !available {
            issues.push(format!("required executable not found on PATH: {name}"));
        }
        tools.push(json!({
            "name": name,
            "required": required_now,
            "requiredForPublish": required_publish,
            "available": available
        }));
    }

    let python = if python_required {
        match python_command() {
            Ok(command) => {
                tools.push(json!({
                    "name": command,
                    "required": !dry_run,
                    "requiredForPublish": true,
                    "available": true,
                    "kind": "python"
                }));
                Some(command)
            }
            Err(error) => {
                if !dry_run {
                    issues.push(error.to_string());
                }
                tools.push(json!({
                    "name": "python3.10+",
                    "required": !dry_run,
                    "requiredForPublish": true,
                    "available": false,
                    "kind": "python"
                }));
                None
            }
        }
    } else {
        None
    };

    for (kind, required_now, required_publish, available) in [
        (
            "npm",
            npm_publish_required,
            selected.contains(&"npm"),
            has_npm_auth(),
        ),
        (
            "cargo",
            cargo_publish_required,
            selected.contains(&"cargo"),
            has_cargo_auth(),
        ),
        (
            "pypi",
            pypi_publish_required,
            selected.contains(&"pypi"),
            has_pypi_auth(),
        ),
    ] {
        if required_now && !available {
            issues.push(format!("missing auth for {kind} publish"));
        }
        auth.push(json!({
            "kind": kind,
            "required": required_now,
            "requiredForPublish": required_publish,
            "available": available
        }));
    }

    Ok(json!({
        "ok": issues.is_empty(),
        "tools": tools,
        "auth": auth,
        "python": python,
        "issues": issues,
        "notes": [
            "publish --plan --json reports local preflight state and helper invocation only",
            "it does not perform live registry propagation or auth validation beyond local presence checks"
        ]
    }))
}

fn resolve_host_target() -> PublishHostInfo {
    let os = env::consts::OS.to_string();
    let arch = env::consts::ARCH.to_string();
    let target = match (env::consts::OS, env::consts::ARCH) {
        ("macos", "aarch64") => "darwin-arm64",
        ("macos", "x86_64") => "darwin-x64",
        ("linux", "x86_64") => "linux-x64",
        ("windows", "x86_64") => "win32-x64",
        _ => "unsupported",
    }
    .to_string();
    PublishHostInfo { target, os, arch }
}

fn command_exists(command: &str) -> bool {
    if cfg!(windows) {
        Command::new(env::var("ComSpec").unwrap_or_else(|_| "cmd.exe".to_string()))
            .args(["/d", "/s", "/c", "where", command])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    } else {
        Command::new("sh")
            .args(["-lc", &format!("command -v \"{command}\"")])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }
}

fn has_npm_auth() -> bool {
    if env::var_os("NPM_TOKEN").is_some() {
        return true;
    }
    let status = if cfg!(windows) {
        Command::new(env::var("ComSpec").unwrap_or_else(|_| "cmd.exe".to_string()))
            .args(["/d", "/s", "/c", "npm", "whoami"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
    } else {
        Command::new("npm")
            .args(["whoami"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
    };
    status.map(|entry| entry.success()).unwrap_or(false)
}

fn has_cargo_auth() -> bool {
    if env::var_os("CARGO_REGISTRY_TOKEN").is_some() {
        return true;
    }
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from));
    if let Some(home) = home {
        return home.join(".cargo/credentials.toml").is_file()
            || home.join(".cargo/credentials").is_file();
    }
    false
}

fn has_pypi_auth() -> bool {
    if env::var_os("TWINE_USERNAME").is_some() && env::var_os("TWINE_PASSWORD").is_some() {
        return true;
    }
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from));
    if let Some(home) = home {
        return home.join(".pypirc").is_file();
    }
    false
}

fn python_command() -> Result<String> {
    for candidate in [
        "python3.14",
        "python3.13",
        "python3.12",
        "python3.11",
        "python3.10",
        "python3",
        "python",
    ] {
        let status = Command::new(candidate)
            .args([
                "-c",
                "import sys; raise SystemExit(0 if sys.version_info >= (3, 10) else 1)",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if status.map(|entry| entry.success()).unwrap_or(false) {
            return Ok(candidate.to_string());
        }
    }
    bail!("no Python 3.10+ interpreter found on PATH");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io;
    use std::os::unix::process::ExitStatusExt;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn unique_temp_path(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("{prefix}-{unique}-{counter}"))
    }

    fn fixture_root() -> PathBuf {
        let root = unique_temp_path("ossplate-release-fixture");
        fs::create_dir_all(root.join("core-rs/src/scaffold")).unwrap();
        fs::create_dir_all(root.join("core-rs/src/sync")).unwrap();
        fs::create_dir_all(root.join("wrapper-js")).unwrap();
        fs::create_dir_all(root.join("wrapper-py")).unwrap();
        fs::create_dir_all(root.join("scripts")).unwrap();
        fs::write(
            root.join("ossplate.toml"),
            "[project]\nname = \"Ossplate\"\n",
        )
        .unwrap();
        fs::write(
            root.join("scaffold-payload.json"),
            "{\n  \"requiredPaths\": []\n}\n",
        )
        .unwrap();
        fs::write(root.join("README.md"), "# Ossplate\n").unwrap();
        fs::write(
            root.join("core-rs/Cargo.toml"),
            "[package]\nname = \"ossplate\"\n",
        )
        .unwrap();
        fs::write(root.join("core-rs/src/main.rs"), "fn main() {}\n").unwrap();
        fs::write(root.join("core-rs/src/main_tests.rs"), "// main tests\n").unwrap();
        fs::write(
            root.join("core-rs/src/test_support.rs"),
            "// test support\n",
        )
        .unwrap();
        fs::write(root.join("core-rs/src/config.rs"), "// config\n").unwrap();
        fs::write(root.join("core-rs/src/output.rs"), "// output\n").unwrap();
        fs::write(root.join("core-rs/src/release.rs"), "// release\n").unwrap();
        fs::write(root.join("core-rs/src/scaffold.rs"), "// scaffold\n").unwrap();
        fs::write(
            root.join("core-rs/src/scaffold_manifest.rs"),
            "// scaffold manifest\n",
        )
        .unwrap();
        fs::write(
            root.join("core-rs/src/scaffold/identity_application.rs"),
            "// identity\n",
        )
        .unwrap();
        fs::write(
            root.join("core-rs/src/scaffold/projection.rs"),
            "// projection\n",
        )
        .unwrap();
        fs::write(
            root.join("core-rs/src/scaffold/template_root.rs"),
            "// template root\n",
        )
        .unwrap();
        fs::write(
            root.join("core-rs/src/source_checkout.rs"),
            "// source checkout\n",
        )
        .unwrap();
        fs::write(root.join("core-rs/src/sync.rs"), "// sync\n").unwrap();
        fs::write(root.join("core-rs/src/sync/metadata.rs"), "// metadata\n").unwrap();
        fs::write(root.join("core-rs/src/sync/text.rs"), "// text\n").unwrap();
        fs::write(
            root.join("wrapper-js/package.json"),
            "{\n  \"name\": \"ossplate\"\n}\n",
        )
        .unwrap();
        fs::write(
            root.join("wrapper-py/pyproject.toml"),
            "[project]\nname = \"ossplate\"\n",
        )
        .unwrap();
        fs::write(
            root.join("scripts/publish-local.mjs"),
            "console.log('ok')\n",
        )
        .unwrap();
        root
    }

    struct FakePublishHelperRunner {
        status: ExitStatus,
        error: Option<io::Error>,
    }

    impl PublishHelperRunner for FakePublishHelperRunner {
        fn run(&self, _root: &Path, _args: &[String]) -> Result<ExitStatus> {
            if let Some(error) = &self.error {
                return Err(anyhow::anyhow!("{}", error))
                    .context("failed to start local publish helper via node");
            }
            Ok(self.status)
        }
    }

    #[test]
    fn build_publish_args_uses_canonical_root_and_flags() {
        let root = fixture_root();
        let root = root.canonicalize().unwrap();
        let script_path = root.join("scripts/publish-local.mjs");
        let args = build_publish_args(&script_path, &root, true, PublishRegistry::Pypi, true);
        assert_eq!(
            args,
            vec![
                script_path.to_string_lossy().to_string(),
                "--root".to_string(),
                root.to_string_lossy().to_string(),
                "--registry".to_string(),
                "pypi".to_string(),
                "--dry-run".to_string(),
                "--skip-existing".to_string()
            ]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn publish_helper_planning_canonicalizes_root_and_preserves_flags() {
        let root = fixture_root();
        let invocation =
            plan_publish_helper_invocation(&root, true, PublishRegistry::Pypi, true).unwrap();
        assert!(invocation.root.is_absolute());
        assert_eq!(
            invocation.args[2],
            invocation.root.to_string_lossy().to_string()
        );
        assert_eq!(invocation.args[4], "pypi");
        assert!(invocation.args.contains(&"--dry-run".to_string()));
        assert!(invocation.args.contains(&"--skip-existing".to_string()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn publish_requires_publish_helper_script() {
        let root = fixture_root();
        fs::remove_file(root.join("scripts/publish-local.mjs")).unwrap();
        let error = publish_repo(&root, false, PublishRegistry::All, false).unwrap_err();
        assert!(error
            .to_string()
            .contains("publish requires a full scaffold source checkout; missing"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn publish_requires_full_scaffold_source_checkout() {
        let root = unique_temp_path("ossplate-release-incomplete");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("ossplate.toml"),
            "[project]\nname = \"Ossplate\"\n",
        )
        .unwrap();
        let error = publish_repo(&root, false, PublishRegistry::All, false).unwrap_err();
        assert!(error.to_string().contains(
            "publish requires a full scaffold source checkout; missing required scaffold paths"
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn publish_non_zero_helper_exit_maps_to_publish_failed() {
        let root = fixture_root();
        let runner = FakePublishHelperRunner {
            status: ExitStatus::from_raw(1 << 8),
            error: None,
        };
        let error = publish_repo_with_runner(&root, false, PublishRegistry::All, false, &runner)
            .unwrap_err();
        assert!(error.to_string().contains("publish failed"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn publish_runner_failure_keeps_node_launch_context() {
        let root = fixture_root();
        let runner = FakePublishHelperRunner {
            status: ExitStatus::from_raw(0),
            error: Some(io::Error::new(io::ErrorKind::NotFound, "missing node")),
        };
        let error = publish_repo_with_runner(&root, false, PublishRegistry::All, false, &runner)
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("failed to start local publish helper via node"));
        fs::remove_dir_all(root).unwrap();
    }
}
