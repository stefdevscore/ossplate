use crate::sync::sync_repo;
use crate::{load_config, write_config, IdentityOverrides, ToolConfig};
use anyhow::{anyhow, bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn create_scaffold(target: &Path, overrides: &IdentityOverrides) -> Result<()> {
    let source_root = discover_template_root()?;
    ensure_scaffold_source_root(&source_root)?;
    create_scaffold_from(&source_root, target, overrides)
}

pub(crate) fn init_scaffold(target: &Path, overrides: &IdentityOverrides) -> Result<()> {
    let source_root = discover_template_root()?;
    ensure_scaffold_source_root(&source_root)?;
    init_scaffold_from(&source_root, target, overrides)
}

pub(crate) fn create_scaffold_from(
    source_root: &Path,
    target: &Path,
    overrides: &IdentityOverrides,
) -> Result<()> {
    if target.exists() {
        if target.read_dir()?.next().is_some() {
            bail!("target directory is not empty: {}", target.display());
        }
    } else {
        fs::create_dir_all(target)
            .with_context(|| format!("failed to create {}", target.display()))?;
    }

    let source_root = source_root.canonicalize().with_context(|| {
        format!(
            "failed to canonicalize source root {}",
            source_root.display()
        )
    })?;
    let target_root = target
        .canonicalize()
        .with_context(|| format!("failed to canonicalize target root {}", target.display()))?;
    if target_root.starts_with(&source_root) {
        bail!("target directory must not be inside the source template tree");
    }

    copy_tree(&source_root, &target_root)?;
    apply_config_overrides_to_target(&target_root, &source_root, overrides)?;
    sync_repo(&target_root, false)?;
    println!("created scaffold at {}", target_root.display());
    Ok(())
}

pub(crate) fn init_scaffold_from(
    source_root: &Path,
    target: &Path,
    overrides: &IdentityOverrides,
) -> Result<()> {
    if !target.exists() {
        fs::create_dir_all(target)
            .with_context(|| format!("failed to create {}", target.display()))?;
    }

    let source_root = source_root.canonicalize().with_context(|| {
        format!(
            "failed to canonicalize source root {}",
            source_root.display()
        )
    })?;
    let target_root = target
        .canonicalize()
        .with_context(|| format!("failed to canonicalize target root {}", target.display()))?;
    if target_root.starts_with(&source_root) {
        bail!("target directory must not be inside the source template tree");
    }

    ensure_scaffold_layout(&source_root, &target_root)?;
    apply_config_overrides_to_target(&target_root, &source_root, overrides)?;
    sync_repo(&target_root, false)?;
    println!("initialized scaffold at {}", target_root.display());
    Ok(())
}

fn apply_config_overrides_to_target(
    target_root: &Path,
    source_root: &Path,
    overrides: &IdentityOverrides,
) -> Result<()> {
    let mut config = if target_root.join("ossplate.toml").is_file() {
        load_config(target_root)?
    } else {
        load_config(source_root)?
    };

    apply_overrides(&mut config, overrides);
    write_config(target_root, &config)
}

fn apply_overrides(config: &mut ToolConfig, overrides: &IdentityOverrides) {
    if let Some(value) = &overrides.name {
        config.project.name = value.clone();
    }
    if let Some(value) = &overrides.description {
        config.project.description = value.clone();
    }
    if let Some(value) = &overrides.repository {
        config.project.repository = value.clone();
    }
    if let Some(value) = &overrides.license {
        config.project.license = value.clone();
    }
    if let Some(value) = &overrides.author_name {
        config.author.name = value.clone();
    }
    if let Some(value) = &overrides.author_email {
        config.author.email = value.clone();
    }
    if let Some(value) = &overrides.rust_crate {
        config.packages.rust_crate = value.clone();
    }
    if let Some(value) = &overrides.npm_package {
        config.packages.npm_package = value.clone();
    }
    if let Some(value) = &overrides.python_package {
        config.packages.python_package = value.clone();
    }
    if let Some(value) = &overrides.command {
        config.packages.command = value.clone();
    }
}

pub(crate) fn discover_template_root() -> Result<PathBuf> {
    if let Some(explicit) = std::env::var_os("OSSPLATE_TEMPLATE_ROOT") {
        let explicit = PathBuf::from(explicit);
        if explicit.join("ossplate.toml").is_file() {
            return Ok(explicit);
        }
        bail!("OSSPLATE_TEMPLATE_ROOT does not point to a scaffold root containing ossplate.toml");
    }

    let exe = std::env::current_exe().context("failed to determine current executable path")?;
    for ancestor in exe.ancestors() {
        if ancestor.join("ossplate.toml").is_file() {
            return Ok(ancestor.to_path_buf());
        }
    }
    std::env::current_dir()
        .context("failed to determine current directory")?
        .ancestors()
        .find(|ancestor| ancestor.join("ossplate.toml").is_file())
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow!("failed to locate template root containing ossplate.toml"))
}

pub(crate) fn ensure_scaffold_source_root(root: &Path) -> Result<()> {
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
        "create/init require a full scaffold source checkout; missing required scaffold paths: {}",
        missing.join(", ")
    )
}

fn copy_tree(source_root: &Path, target_root: &Path) -> Result<()> {
    for entry in fs::read_dir(source_root)
        .with_context(|| format!("failed to read {}", source_root.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let file_name = entry.file_name();
        if should_skip_copy(&file_name) {
            continue;
        }

        let target_path = target_root.join(&file_name);
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            fs::create_dir_all(&target_path)
                .with_context(|| format!("failed to create {}", target_path.display()))?;
            copy_tree(&source_path, &target_path)?;
        } else if file_type.is_file() {
            fs::copy(&source_path, &target_path).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    source_path.display(),
                    target_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn ensure_scaffold_layout(source_root: &Path, target_root: &Path) -> Result<()> {
    for entry in fs::read_dir(source_root)
        .with_context(|| format!("failed to read {}", source_root.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let file_name = entry.file_name();
        if should_skip_copy(&file_name) {
            continue;
        }

        let target_path = target_root.join(&file_name);
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if !target_path.exists() {
                fs::create_dir_all(&target_path)
                    .with_context(|| format!("failed to create {}", target_path.display()))?;
            }
            ensure_scaffold_layout(&source_path, &target_path)?;
        } else if !target_path.exists() {
            fs::copy(&source_path, &target_path).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    source_path.display(),
                    target_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn should_skip_copy(file_name: &std::ffi::OsStr) -> bool {
    matches!(
        file_name.to_str(),
        Some(".git")
            | Some(".venv")
            | Some("node_modules")
            | Some("target")
            | Some("dist")
            | Some("build")
            | Some("__pycache__")
    )
}
