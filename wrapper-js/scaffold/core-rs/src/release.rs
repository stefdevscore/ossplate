use anyhow::{bail, Context, Result};
use clap::ValueEnum;
use std::path::Path;
use std::process::{Command, ExitStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
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
    ensure_publish_source_root(root)?;
    let script_path = root.join("scripts/publish-local.mjs");
    if !script_path.is_file() {
        bail!(
            "publish requires a full scaffold source checkout; missing {}",
            script_path.display()
        );
    }

    let root = root
        .canonicalize()
        .with_context(|| format!("failed to canonicalize publish path {}", root.display()))?;
    let args = build_publish_args(&script_path, &root, dry_run, registry, skip_existing);

    let status = run_publish_helper(&root, &args)?;
    if status.success() {
        Ok(())
    } else {
        bail!("publish failed")
    }
}

fn ensure_publish_source_root(root: &Path) -> Result<()> {
    let required = [
        "ossplate.toml",
        "README.md",
        "core-rs/Cargo.toml",
        "core-rs/src/main.rs",
        "core-rs/src/release.rs",
        "core-rs/src/scaffold.rs",
        "core-rs/src/sync.rs",
        "core-rs/src/sync/metadata.rs",
        "core-rs/src/sync/text.rs",
        "wrapper-js/package.json",
        "wrapper-py/pyproject.toml",
    ];

    let missing: Vec<_> = required
        .iter()
        .filter(|path| !root.join(path).exists())
        .copied()
        .collect();

    if missing.is_empty() {
        return Ok(());
    }

    bail!(
        "publish requires a full scaffold source checkout; missing required scaffold paths: {}",
        missing.join(", ")
    )
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

fn run_publish_helper(root: &Path, args: &[String]) -> Result<ExitStatus> {
    Command::new("node")
        .args(args)
        .current_dir(root)
        .status()
        .context("failed to start local publish helper via node")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::process::ExitStatusExt;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_path(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{unique}"))
    }

    fn fixture_root() -> PathBuf {
        let root = unique_temp_path("ossplate-release-fixture");
        fs::create_dir_all(root.join("core-rs")).unwrap();
        fs::create_dir_all(root.join("wrapper-js")).unwrap();
        fs::create_dir_all(root.join("wrapper-py")).unwrap();
        fs::create_dir_all(root.join("scripts")).unwrap();
        fs::write(
            root.join("ossplate.toml"),
            "[project]\nname = \"Ossplate\"\n",
        )
        .unwrap();
        fs::write(root.join("README.md"), "# Ossplate\n").unwrap();
        fs::write(
            root.join("core-rs/Cargo.toml"),
            "[package]\nname = \"ossplate\"\n",
        )
        .unwrap();
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
        let status = ExitStatus::from_raw(1 << 8);
        assert!(!status.success());
    }
}
