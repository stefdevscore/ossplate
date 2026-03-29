use anyhow::{Context, Result};
use clap::Args;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default, Args)]
pub(crate) struct IdentityOverrides {
    #[arg(long)]
    pub(crate) name: Option<String>,
    #[arg(long)]
    pub(crate) description: Option<String>,
    #[arg(long)]
    pub(crate) repository: Option<String>,
    #[arg(long)]
    pub(crate) license: Option<String>,
    #[arg(long = "author-name")]
    pub(crate) author_name: Option<String>,
    #[arg(long = "author-email")]
    pub(crate) author_email: Option<String>,
    #[arg(long = "rust-crate")]
    pub(crate) rust_crate: Option<String>,
    #[arg(long = "npm-package")]
    pub(crate) npm_package: Option<String>,
    #[arg(long = "python-package")]
    pub(crate) python_package: Option<String>,
    #[arg(long)]
    pub(crate) command: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct ToolConfig {
    pub(crate) project: ProjectConfig,
    pub(crate) author: AuthorConfig,
    pub(crate) packages: PackageConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct ProjectConfig {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) repository: String,
    pub(crate) license: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct AuthorConfig {
    pub(crate) name: String,
    pub(crate) email: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct PackageConfig {
    pub(crate) rust_crate: String,
    pub(crate) npm_package: String,
    pub(crate) python_package: String,
    pub(crate) command: String,
}

pub(crate) fn load_config(root: &Path) -> Result<ToolConfig> {
    let contents =
        fs::read_to_string(root.join("ossplate.toml")).context("failed to read ossplate.toml")?;
    toml::from_str(&contents).context("failed to parse ossplate.toml")
}

pub(crate) fn write_config(root: &Path, config: &ToolConfig) -> Result<()> {
    let mut rendered = toml::to_string(config).context("failed to serialize ossplate.toml")?;
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    fs::write(root.join("ossplate.toml"), rendered).context("failed to write ossplate.toml")?;
    Ok(())
}
