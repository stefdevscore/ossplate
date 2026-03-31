use anyhow::{bail, Context, Result};
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

pub(crate) fn prepare_create_target(source_root: &Path, target: &Path) -> Result<()> {
    if target.exists() {
        if target.read_dir()?.next().is_some() {
            bail!("target directory is not empty: {}", target.display());
        }
    } else {
        fs::create_dir_all(target)
            .with_context(|| format!("failed to create {}", target.display()))?;
    }

    ensure_target_outside_source_tree(source_root, target)
}

pub(crate) fn prepare_init_target(source_root: &Path, target: &Path) -> Result<()> {
    if !target.exists() {
        fs::create_dir_all(target)
            .with_context(|| format!("failed to create {}", target.display()))?;
    }

    ensure_target_outside_source_tree(source_root, target)
}

fn ensure_target_outside_source_tree(source_root: &Path, target: &Path) -> Result<()> {
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
    Ok(())
}

pub(crate) fn copy_tree(source_root: &Path, target_root: &Path) -> Result<()> {
    for entry in fs::read_dir(source_root)
        .with_context(|| format!("failed to read {}", source_root.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let file_name = projected_file_name(entry.file_name());
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

pub(crate) fn ensure_scaffold_layout(source_root: &Path, target_root: &Path) -> Result<()> {
    for entry in fs::read_dir(source_root)
        .with_context(|| format!("failed to read {}", source_root.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let file_name = projected_file_name(entry.file_name());
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

fn projected_file_name(file_name: std::ffi::OsString) -> std::ffi::OsString {
    if file_name == OsStr::new(".npmignore") {
        OsStr::new(".gitignore").to_os_string()
    } else {
        file_name
    }
}

fn should_skip_copy(file_name: &OsStr) -> bool {
    matches!(
        file_name.to_str(),
        Some(".git")
            | Some(".venv")
            | Some(".dist-assets")
            | Some(".live-e2e")
            | Some("node_modules")
            | Some("target")
            | Some("dist")
            | Some("build")
            | Some("__pycache__")
            | Some(".tmp-inspect")
            | Some("generated-embedded-template-root")
            | Some(".tmp-build-venv")
            | Some(".tmp-wheel-venv")
            | Some(".tmp-wheel-created")
    )
}
