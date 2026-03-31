mod identity_application;
mod projection;
mod template_root;

use crate::config::IdentityOverrides;
use crate::config::{is_template_project, load_config};
use crate::output::render_bootstrap_output;
use crate::sync::sync_repo;
use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

use identity_application::apply_config_overrides_to_target;
use projection::{copy_tree, ensure_scaffold_layout, prepare_create_target, prepare_init_target};
pub(crate) use template_root::{discover_template_root, ensure_scaffold_source_root};

pub(crate) fn create_scaffold(target: &Path, overrides: &IdentityOverrides) -> Result<()> {
    let source_root = discover_template_root()?;
    ensure_scaffold_source_root(&source_root)?;
    finalize_scaffold_from(&source_root, target, overrides, "create", false)?;
    Ok(())
}

pub(crate) fn init_scaffold(target: &Path, overrides: &IdentityOverrides) -> Result<()> {
    let source_root = discover_template_root()?;
    ensure_scaffold_source_root(&source_root)?;
    finalize_scaffold_from(&source_root, target, overrides, "init", false)?;
    Ok(())
}

#[cfg(test)]
pub(crate) fn create_scaffold_from(
    source_root: &Path,
    target: &Path,
    overrides: &IdentityOverrides,
) -> Result<()> {
    finalize_scaffold_from(source_root, target, overrides, "create", false).map(|_| ())
}

#[cfg(test)]
pub(crate) fn init_scaffold_from(
    source_root: &Path,
    target: &Path,
    overrides: &IdentityOverrides,
) -> Result<()> {
    finalize_scaffold_from(source_root, target, overrides, "init", false).map(|_| ())
}

pub(crate) fn create_scaffold_json(target: &Path, overrides: &IdentityOverrides) -> Result<String> {
    let source_root = discover_template_root()?;
    ensure_scaffold_source_root(&source_root)?;
    let (target_root, config) =
        finalize_scaffold_from(&source_root, target, overrides, "create", true)?;
    render_bootstrap_output("create", &target_root, config)
}

pub(crate) fn init_scaffold_json(target: &Path, overrides: &IdentityOverrides) -> Result<String> {
    let source_root = discover_template_root()?;
    ensure_scaffold_source_root(&source_root)?;
    let (target_root, config) =
        finalize_scaffold_from(&source_root, target, overrides, "init", true)?;
    render_bootstrap_output("init", &target_root, config)
}

fn finalize_scaffold_from(
    source_root: &Path,
    target: &Path,
    overrides: &IdentityOverrides,
    action: &str,
    quiet: bool,
) -> Result<(std::path::PathBuf, crate::config::ToolConfig)> {
    match action {
        "create" => prepare_create_target(source_root, target)?,
        "init" => prepare_init_target(source_root, target)?,
        _ => unreachable!("unsupported scaffold action"),
    }
    let source_root = source_root.canonicalize()?;
    let target_root = target.canonicalize()?;
    if action == "create" {
        copy_tree(&source_root, &target_root)?;
    } else {
        ensure_scaffold_layout(&source_root, &target_root)?;
    }
    apply_config_overrides_to_target(&target_root, &source_root, overrides, action)?;
    sync_repo_with_output(&target_root, false, quiet)?;
    let config = load_config(&target_root)?;
    remove_template_only_paths(&target_root, &config)?;
    prune_template_only_manifest_paths(&target_root, &config)?;
    project_embedded_template_root(&target_root)?;
    if !quiet {
        match action {
            "create" => println!("created scaffold at {}", target_root.display()),
            "init" => println!("initialized scaffold at {}", target_root.display()),
            _ => {}
        }
    }
    Ok((target_root, config))
}

fn sync_repo_with_output(root: &Path, check: bool, quiet: bool) -> Result<()> {
    if quiet {
        crate::sync::sync_repo_quiet(root, check)
    } else {
        sync_repo(root, check)
    }
}

fn project_embedded_template_root(root: &Path) -> Result<()> {
    let config = load_config(root)?;
    let embedded_root = root.join("core-rs/embedded-template-root");
    if embedded_root.exists() {
        fs::remove_dir_all(&embedded_root)
            .with_context(|| format!("failed to clear {}", embedded_root.display()))?;
    }
    fs::create_dir_all(&embedded_root)
        .with_context(|| format!("failed to create {}", embedded_root.display()))?;

    let mut required_paths = vec![
        "ossplate.toml".to_string(),
        "scaffold-payload.json".to_string(),
        "source-checkout.json".to_string(),
    ];
    let template_only_paths = template_only_paths(root)?;
    for manifest_name in ["scaffold-payload.json", "source-checkout.json"] {
        let manifest: Value =
            serde_json::from_str(&fs::read_to_string(root.join(manifest_name)).with_context(
                || format!("failed to read {}", root.join(manifest_name).display()),
            )?)
            .with_context(|| format!("failed to parse {}", root.join(manifest_name).display()))?;
        let paths = manifest
            .get("requiredPaths")
            .and_then(serde_json::Value::as_array)
            .with_context(|| format!("missing requiredPaths in {}", manifest_name))?;
        for path in paths {
            let relative_path = path
                .as_str()
                .with_context(|| format!("non-string required path in {}", manifest_name))?;
            if !relative_path.starts_with("core-rs/") {
                required_paths.push(relative_path.to_string());
            }
        }
    }
    if !is_template_project(&config) {
        required_paths.retain(|path| !template_only_paths.contains(path));
    }
    required_paths.sort();
    required_paths.dedup();

    for relative_path in required_paths {
        if root.join(&relative_path).exists() {
            copy_path(root, &embedded_root, &relative_path)?;
        }
    }

    for relative_path in collect_core_embedded_paths(root)? {
        copy_path(root, &embedded_root, &relative_path)?;
    }

    Ok(())
}

fn remove_template_only_paths(root: &Path, config: &crate::config::ToolConfig) -> Result<()> {
    if is_template_project(config) {
        return Ok(());
    }

    for relative_path in template_only_paths(root)? {
        let target = root.join(&relative_path);
        if target.exists() {
            if target.is_dir() {
                fs::remove_dir_all(&target)
                    .with_context(|| format!("failed to remove {}", target.display()))?;
            } else {
                fs::remove_file(&target)
                    .with_context(|| format!("failed to remove {}", target.display()))?;
            }
        }
    }
    Ok(())
}

fn prune_template_only_manifest_paths(
    root: &Path,
    config: &crate::config::ToolConfig,
) -> Result<()> {
    if is_template_project(config) {
        return Ok(());
    }

    let template_only_paths = template_only_paths(root)?;

    for relative_path in ["scaffold-payload.json", "source-checkout.json"] {
        let manifest_path = root.join(relative_path);
        let mut manifest: Value = serde_json::from_str(
            &fs::read_to_string(&manifest_path)
                .with_context(|| format!("failed to read {}", manifest_path.display()))?,
        )
        .with_context(|| format!("failed to parse {}", manifest_path.display()))?;

        let required_paths = manifest
            .get_mut("requiredPaths")
            .and_then(serde_json::Value::as_array_mut)
            .with_context(|| format!("missing requiredPaths in {}", manifest_path.display()))?;
        required_paths.retain(|entry| {
            entry
                .as_str()
                .is_none_or(|path| !template_only_paths.contains(path))
        });

        let mut rendered = serde_json::to_string_pretty(&manifest)?;
        rendered.push('\n');
        fs::write(&manifest_path, rendered)
            .with_context(|| format!("failed to write {}", manifest_path.display()))?;
    }

    Ok(())
}

fn template_only_paths(root: &Path) -> Result<std::collections::HashSet<String>> {
    let scaffold_payload = manifest_template_only_paths(&root.join("scaffold-payload.json"))?;
    let source_checkout = manifest_template_only_paths(&root.join("source-checkout.json"))?;
    if scaffold_payload != source_checkout {
        anyhow::bail!(
            "templateOnlyPaths must match between scaffold-payload.json and source-checkout.json"
        );
    }
    Ok(scaffold_payload)
}

fn manifest_template_only_paths(manifest_path: &Path) -> Result<std::collections::HashSet<String>> {
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(manifest_path)
            .with_context(|| format!("failed to read {}", manifest_path.display()))?,
    )
    .with_context(|| format!("failed to parse {}", manifest_path.display()))?;

    let paths = manifest
        .get("templateOnlyPaths")
        .and_then(Value::as_array)
        .with_context(|| format!("missing templateOnlyPaths in {}", manifest_path.display()))?;

    paths
        .iter()
        .map(|entry| {
            entry.as_str().map(str::to_string).with_context(|| {
                format!(
                    "non-string templateOnlyPaths entry in {}",
                    manifest_path.display()
                )
            })
        })
        .collect()
}

fn collect_core_embedded_paths(root: &Path) -> Result<Vec<String>> {
    let mut relative_paths = vec![
        "core-rs/Cargo.toml".to_string(),
        "core-rs/Cargo.lock".to_string(),
        "core-rs/build.rs".to_string(),
        "core-rs/runtime-targets.json".to_string(),
        "core-rs/source-checkout.json".to_string(),
    ];
    let src_root = root.join("core-rs/src");
    collect_core_src_paths(root, &src_root, &mut relative_paths)?;
    Ok(relative_paths)
}

fn collect_core_src_paths(
    root: &Path,
    current: &Path,
    relative_paths: &mut Vec<String>,
) -> Result<()> {
    for entry in
        fs::read_dir(current).with_context(|| format!("failed to read {}", current.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_core_src_paths(root, &path, relative_paths)?;
        } else if file_type.is_file() {
            relative_paths.push(
                path.strip_prefix(root)
                    .expect("core source path must be inside the scaffold root")
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
    Ok(())
}

fn copy_path(root: &Path, destination_root: &Path, relative_path: &str) -> Result<()> {
    let source = root.join(relative_path);
    let destination = destination_root.join(relative_path);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    if source.is_dir() {
        crate::scaffold::projection::copy_tree(&source, &destination)?;
    } else {
        fs::copy(&source, &destination).with_context(|| {
            format!(
                "failed to copy {} to {}",
                source.display(),
                destination.display()
            )
        })?;
    }
    Ok(())
}
