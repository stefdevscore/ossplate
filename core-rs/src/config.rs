use anyhow::{Context, Result};
use clap::Args;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub(crate) const GENERATED_REPOSITORY_PLACEHOLDER: &str =
    "https://example.com/replace-with-your-repository";
pub(crate) const GENERATED_AUTHOR_NAME_PLACEHOLDER: &str = "TODO: set author name";
pub(crate) const GENERATED_AUTHOR_EMAIL_PLACEHOLDER: &str = "you@example.com";

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
    pub(crate) metadata: MetadataConfig,
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct MetadataConfig {
    pub(crate) rust_keywords: Vec<String>,
    pub(crate) rust_categories: Vec<String>,
    pub(crate) npm_keywords: Vec<String>,
    pub(crate) python_keywords: Vec<String>,
    pub(crate) python_classifiers: Vec<String>,
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

pub(crate) fn generated_project_description(command: &str) -> String {
    format!("Ship the `{command}` CLI through Cargo, npm, and PyPI. Replace this description before release.")
}

pub(crate) fn is_template_project(config: &ToolConfig) -> bool {
    config.packages.command == "ossplate"
        && config.packages.rust_crate == "ossplate"
        && config.packages.npm_package == "ossplate"
        && config.packages.python_package == "ossplate"
        && config.project.repository == "https://github.com/stefdevscore/ossplate"
        && config.author.name == "Stef"
        && config.author.email == "stefdevscore@github.com"
}

pub(crate) fn generated_metadata_warnings(config: &ToolConfig) -> Vec<String> {
    if is_template_project(config) {
        return Vec::new();
    }

    let mut warnings = Vec::new();
    if config.project.description == generated_project_description(&config.packages.command) {
        warnings.push(
            "project.description still uses the generated placeholder; replace it before release"
                .to_string(),
        );
    }
    if config.project.repository == GENERATED_REPOSITORY_PLACEHOLDER {
        warnings.push(
            "project.repository still uses the generated placeholder; set the real repository URL before release"
                .to_string(),
        );
    }
    if config.author.name == GENERATED_AUTHOR_NAME_PLACEHOLDER {
        warnings.push(
            "author.name still uses the generated placeholder; set the real maintainer name before release"
                .to_string(),
        );
    }
    if config.author.email == GENERATED_AUTHOR_EMAIL_PLACEHOLDER {
        warnings.push(
            "author.email still uses the generated placeholder; set the real maintainer email before release"
                .to_string(),
        );
    }
    warnings
}
