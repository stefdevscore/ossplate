mod identity_application;
mod projection;
mod template_root;

use crate::config::load_config;
use crate::config::IdentityOverrides;
use crate::sync::sync_repo;
use anyhow::Result;
use std::path::Path;

use identity_application::apply_config_overrides_to_target;
use projection::{
    copy_tree, ensure_scaffold_layout, prepare_create_target, prepare_init_target,
    stage_scaffold_mirrors,
};
pub(crate) use template_root::{discover_template_root, ensure_scaffold_source_root};

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
    prepare_create_target(source_root, target)?;
    let source_root = source_root.canonicalize()?;
    let target_root = target.canonicalize()?;
    copy_tree(&source_root, &target_root)?;
    apply_config_overrides_to_target(&target_root, &source_root, overrides)?;
    sync_repo(&target_root, false)?;
    let config = load_config(&target_root)?;
    stage_scaffold_mirrors(&target_root, &config)?;
    println!("created scaffold at {}", target_root.display());
    Ok(())
}

pub(crate) fn init_scaffold_from(
    source_root: &Path,
    target: &Path,
    overrides: &IdentityOverrides,
) -> Result<()> {
    prepare_init_target(source_root, target)?;
    let source_root = source_root.canonicalize()?;
    let target_root = target.canonicalize()?;
    ensure_scaffold_layout(&source_root, &target_root)?;
    apply_config_overrides_to_target(&target_root, &source_root, overrides)?;
    sync_repo(&target_root, false)?;
    let config = load_config(&target_root)?;
    stage_scaffold_mirrors(&target_root, &config)?;
    println!("initialized scaffold at {}", target_root.display());
    Ok(())
}
