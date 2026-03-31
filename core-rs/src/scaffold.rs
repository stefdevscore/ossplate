mod identity_application;
mod projection;
mod template_root;

use crate::config::load_config;
use crate::config::IdentityOverrides;
use crate::output::render_bootstrap_output;
use crate::sync::sync_repo;
use anyhow::Result;
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
    apply_config_overrides_to_target(&target_root, &source_root, overrides)?;
    sync_repo_with_output(&target_root, false, quiet)?;
    let config = load_config(&target_root)?;
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
