use crate::source_checkout::ensure_source_checkout;
use anyhow::{bail, Context, Result};
use clap::ValueEnum;
use std::path::Path;
use std::process::{Command, ExitStatus};

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
    crate::output::render_publish_plan_output(crate::output::PublishPlanOutput {
        ok: true,
        root: invocation.root.display().to_string(),
        registry: invocation.registry,
        dry_run: invocation.dry_run,
        skip_existing: invocation.skip_existing,
        helper: invocation.helper.display().to_string(),
        argv: invocation.args,
    })
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
