use crate::config::{load_config, write_config, IdentityOverrides, ToolConfig};
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn apply_config_overrides_to_target(
    target_root: &Path,
    source_root: &Path,
    overrides: &IdentityOverrides,
) -> Result<()> {
    let mut config = if target_root.join("ossplate.toml").is_file() {
        load_config(target_root)?
    } else {
        load_config(source_root)?
    };
    let original = config.clone();

    apply_overrides(&mut config, overrides);
    relocate_generated_identity_paths(target_root, &original, &config)?;
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
    if let Some(value) = &overrides.command {
        config.packages.command = value.clone();
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
    if overrides.command.is_some() && overrides.rust_crate.is_none() {
        config.packages.rust_crate = config.packages.command.clone();
    }
    if overrides.command.is_some() && overrides.npm_package.is_none() {
        config.packages.npm_package = config.packages.command.clone();
    }
    if overrides.command.is_some() && overrides.python_package.is_none() {
        config.packages.python_package = config.packages.command.clone();
    }
}

fn relocate_generated_identity_paths(
    target_root: &Path,
    original: &ToolConfig,
    updated: &ToolConfig,
) -> Result<()> {
    relocate_path(
        target_root,
        wrapper_js_launcher_path(original),
        wrapper_js_launcher_path(updated),
    )?;
    relocate_path(
        target_root,
        wrapper_py_package_dir(original),
        wrapper_py_package_dir(updated),
    )?;
    Ok(())
}

fn relocate_path(target_root: &Path, original: PathBuf, updated: PathBuf) -> Result<()> {
    if original == updated {
        return Ok(());
    }

    let original_path = target_root.join(&original);
    let updated_path = target_root.join(&updated);
    if !original_path.exists() || updated_path.exists() {
        return Ok(());
    }

    if let Some(parent) = updated_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(original_path, updated_path)?;
    Ok(())
}

fn wrapper_js_launcher_path(config: &ToolConfig) -> PathBuf {
    PathBuf::from("wrapper-js")
        .join("bin")
        .join(format!("{}.js", config.packages.command))
}

fn wrapper_py_package_dir(config: &ToolConfig) -> PathBuf {
    PathBuf::from("wrapper-py")
        .join("src")
        .join(python_module_name(config))
}

fn python_module_name(config: &ToolConfig) -> String {
    config
        .packages
        .python_package
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => ch,
            '-' | '.' => '_',
            _ => '_',
        })
        .collect()
}
