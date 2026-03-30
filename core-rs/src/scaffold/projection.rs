use crate::config::ToolConfig;
use anyhow::{anyhow, bail, Context, Result};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

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

pub(crate) fn stage_scaffold_mirrors(target_root: &Path, config: &ToolConfig) -> Result<()> {
    let payload =
        fs::read_to_string(target_root.join("scaffold-payload.json")).with_context(|| {
            format!(
                "failed to read {}",
                target_root.join("scaffold-payload.json").display()
            )
        })?;
    let manifest: serde_json::Value =
        serde_json::from_str(&payload).context("failed to parse scaffold-payload.json")?;
    let required_paths = manifest["requiredPaths"]
        .as_array()
        .ok_or_else(|| anyhow!("scaffold-payload.json is missing requiredPaths"))?;

    for destination_root in scaffold_mirror_roots(target_root, config) {
        remove_tree_if_exists(&destination_root)?;
        fs::create_dir_all(&destination_root)
            .with_context(|| format!("failed to create {}", destination_root.display()))?;

        for relative_path in required_paths {
            let relative_path = relative_path.as_str().ok_or_else(|| {
                anyhow!("scaffold-payload.json contains a non-string required path")
            })?;
            let source_path = target_root.join(relative_path);
            if !source_path.exists() {
                bail!("required scaffold path is missing: {relative_path}");
            }

            let destination_path = destination_root.join(relative_path);
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }
            copy_path(&source_path, &destination_path)?;
        }
    }

    Ok(())
}

fn scaffold_mirror_roots(target_root: &Path, config: &ToolConfig) -> [PathBuf; 2] {
    [
        target_root.join("wrapper-js").join("scaffold"),
        target_root
            .join("wrapper-py")
            .join("src")
            .join(python_module_name(&config.packages.python_package))
            .join("scaffold"),
    ]
}

fn python_module_name(package_name: &str) -> String {
    package_name
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => ch,
            '-' | '.' => '_',
            _ => '_',
        })
        .collect()
}

fn copy_path(source_path: &Path, destination_path: &Path) -> Result<()> {
    if source_path.is_dir() {
        fs::create_dir_all(destination_path)
            .with_context(|| format!("failed to create {}", destination_path.display()))?;
        for entry in fs::read_dir(source_path)
            .with_context(|| format!("failed to read {}", source_path.display()))?
        {
            let entry = entry?;
            copy_path(&entry.path(), &destination_path.join(entry.file_name()))?;
        }
    } else {
        fs::copy(source_path, destination_path).with_context(|| {
            format!(
                "failed to copy {} to {}",
                source_path.display(),
                destination_path.display()
            )
        })?;
    }
    Ok(())
}

fn remove_tree_if_exists(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_dir_all(path).with_context(|| format!("failed to remove {}", path.display()))?;
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
            | Some(".tmp-build-venv")
            | Some(".tmp-wheel-venv")
            | Some(".tmp-wheel-created")
    )
}
