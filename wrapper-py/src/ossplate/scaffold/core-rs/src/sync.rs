mod metadata;
mod text;

use crate::config::{load_config, ToolConfig};
use anyhow::{anyhow, bail, Context, Result};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub(crate) use metadata::runtime_package_managed_files;
pub(crate) use text::{format_human_issues, issue};

#[cfg(test)]
pub(crate) use text::{github_blob_url, github_raw_url, render_wrapper_readme};

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct ValidationOutput {
    pub(crate) ok: bool,
    pub(crate) issues: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub(crate) struct ValidationIssue {
    pub(crate) code: String,
    pub(crate) file: String,
    pub(crate) message: String,
    pub(crate) expected: Option<String>,
    pub(crate) actual: Option<String>,
}

#[derive(Debug, Clone)]
struct SyncChange {
    path: &'static str,
    issues: Vec<ValidationIssue>,
    synced: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ManagedFile {
    pub(crate) path: &'static str,
    pub(crate) validate: fn(&ToolConfig, &str) -> Result<Vec<ValidationIssue>>,
    pub(crate) sync: fn(&ToolConfig, &str) -> Result<String>,
}

pub(crate) fn validate_repo(root: &Path) -> Result<ValidationOutput> {
    let config = load_config(root)?;
    let current = collect_current_files(root)?;
    let mut issues = Vec::new();

    for file in &managed_files() {
        let actual = current
            .get(file.path)
            .ok_or_else(|| anyhow!("missing owned file {}", file.path))?;
        issues.extend((file.validate)(&config, actual)?);
    }

    Ok(ValidationOutput {
        ok: issues.is_empty(),
        issues,
    })
}

pub(crate) fn sync_repo(root: &Path, check: bool) -> Result<()> {
    let drifted = build_sync_changes(root)?;

    if check {
        if drifted.is_empty() {
            println!("sync check ok");
            return Ok(());
        }
        let issues = drifted
            .iter()
            .flat_map(|change| change.issues.iter().cloned())
            .collect::<Vec<_>>();
        println!("{}", format_human_issues("sync check failed:", &issues));
        bail!("sync check failed")
    }

    for change in drifted {
        let target = root.join(change.path);
        fs::write(&target, change.synced)
            .with_context(|| format!("failed to write {}", target.display()))?;
    }

    println!("sync complete");
    Ok(())
}

fn build_sync_changes(root: &Path) -> Result<Vec<SyncChange>> {
    let config = load_config(root)?;
    let current = collect_current_files(root)?;
    let mut drifted = Vec::new();

    for file in &managed_files() {
        let actual = current
            .get(file.path)
            .ok_or_else(|| anyhow!("missing owned file {}", file.path))?;
        let issues = (file.validate)(&config, actual)?;
        if !issues.is_empty() {
            drifted.push(SyncChange {
                path: file.path,
                synced: (file.sync)(&config, actual)?,
                issues,
            });
        }
    }

    Ok(drifted)
}

fn collect_current_files(root: &Path) -> Result<BTreeMap<&'static str, String>> {
    let mut files = BTreeMap::new();
    for file in managed_files() {
        let path = file.path;
        files.insert(
            path,
            fs::read_to_string(root.join(path))
                .with_context(|| format!("failed to read {}", root.join(path).display()))?,
        );
    }
    Ok(files)
}

pub(crate) fn managed_files() -> Vec<ManagedFile> {
    let mut files = vec![
        ManagedFile {
            path: "README.md",
            validate: text::validate_root_readme,
            sync: text::sync_root_readme,
        },
        ManagedFile {
            path: ".github/workflows/ci.yml",
            validate: text::validate_ci_workflow,
            sync: text::sync_ci_workflow,
        },
        ManagedFile {
            path: ".github/workflows/publish.yml",
            validate: text::validate_publish_workflow,
            sync: text::sync_publish_workflow,
        },
        ManagedFile {
            path: ".github/workflows/publish-npm.yml",
            validate: text::validate_publish_npm_workflow,
            sync: text::sync_publish_npm_workflow,
        },
        ManagedFile {
            path: "core-rs/Cargo.toml",
            validate: metadata::validate_cargo_toml,
            sync: metadata::sync_cargo_toml,
        },
        ManagedFile {
            path: "wrapper-js/package.json",
            validate: metadata::validate_package_json,
            sync: metadata::sync_package_json,
        },
        ManagedFile {
            path: "wrapper-py/pyproject.toml",
            validate: metadata::validate_pyproject,
            sync: metadata::sync_pyproject,
        },
        ManagedFile {
            path: "wrapper-js/README.md",
            validate: text::validate_js_readme,
            sync: text::sync_js_readme,
        },
        ManagedFile {
            path: "wrapper-py/README.md",
            validate: text::validate_py_readme,
            sync: text::sync_py_readme,
        },
    ];
    files.extend(runtime_package_managed_files());
    files
}
