use crate::config::{load_config, write_config, IdentityOverrides, ToolConfig};
use anyhow::Result;
use std::path::Path;

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
