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

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct SyncChangePlan {
    pub(crate) path: String,
    pub(crate) synced: String,
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
    sync_repo_internal(root, check, false)
}

pub(crate) fn sync_repo_quiet(root: &Path, check: bool) -> Result<()> {
    sync_repo_internal(root, check, true)
}

fn sync_repo_internal(root: &Path, check: bool, quiet: bool) -> Result<()> {
    let drifted = build_sync_changes(root)?;

    if check {
        if drifted.is_empty() {
            if !quiet {
                println!("sync check ok");
            }
            return Ok(());
        }
        let issues = drifted
            .iter()
            .flat_map(|change| change.issues.iter().cloned())
            .collect::<Vec<_>>();
        if !quiet {
            println!("{}", format_human_issues("sync check failed:", &issues));
        }
        bail!("sync check failed")
    }

    for change in drifted {
        let target = root.join(change.path);
        fs::write(&target, change.synced)
            .with_context(|| format!("failed to write {}", target.display()))?;
    }

    if !quiet {
        println!("sync complete");
    }
    Ok(())
}

pub(crate) fn sync_check_json(root: &Path) -> Result<String> {
    let drifted = build_sync_changes(root)?;
    let issues = drifted
        .iter()
        .flat_map(|change| change.issues.iter().cloned())
        .collect::<Vec<_>>();
    let changes = drifted
        .into_iter()
        .map(|change| SyncChangePlan {
            path: change.path.to_string(),
            synced: change.synced,
        })
        .collect();
    crate::output::render_sync_output("check", issues, changes, false)
}

pub(crate) fn sync_plan_json(root: &Path) -> Result<String> {
    let drifted = build_sync_changes(root)?;
    let issues = drifted
        .iter()
        .flat_map(|change| change.issues.iter().cloned())
        .collect::<Vec<_>>();
    let changes = drifted
        .into_iter()
        .map(|change| SyncChangePlan {
            path: change.path.to_string(),
            synced: change.synced,
        })
        .collect();
    crate::output::render_sync_output("plan", issues, changes, true)
}

pub(crate) fn sync_apply_json(root: &Path) -> Result<String> {
    let drifted = build_sync_changes(root)?;
    let issues = drifted
        .iter()
        .flat_map(|change| change.issues.iter().cloned())
        .collect::<Vec<_>>();
    let changes = drifted
        .iter()
        .map(|change| SyncChangePlan {
            path: change.path.to_string(),
            synced: change.synced.clone(),
        })
        .collect::<Vec<_>>();
    for change in drifted {
        let target = root.join(change.path);
        fs::write(&target, change.synced)
            .with_context(|| format!("failed to write {}", target.display()))?;
    }
    crate::output::render_sync_output("apply", issues, changes, true)
}

pub(crate) fn inspect_repo_json(root: &Path) -> Result<String> {
    let config = load_config(root)?;
    let managed_files = managed_files()
        .into_iter()
        .map(|file| file.path.to_string())
        .collect();
    let runtime_targets = read_json(root, "runtime-targets.json")?;
    let scaffold_payload = read_json(root, "scaffold-payload.json")?;
    let source_checkout = read_optional_json(root, "source-checkout.json")?;
    let derived = build_inspect_derived(&config, &runtime_targets)?;
    crate::output::render_inspect_output(crate::output::InspectOutput {
        config,
        managed_files,
        runtime_targets,
        scaffold_payload,
        source_checkout,
        derived,
    })
}

fn build_inspect_derived(
    config: &ToolConfig,
    runtime_targets: &serde_json::Value,
) -> Result<serde_json::Value> {
    let python_module = python_module_name(&config.packages.python_package);
    let js_launcher = format!("wrapper-js/bin/{}.js", config.packages.command);
    let python_package_dir = format!("wrapper-py/src/{python_module}");
    let source_root = format!("{python_package_dir}/cli.py");
    let runtime_packages = runtime_targets
        .get("targets")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|target| {
            let target_name = target
                .get("target")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let folder_suffix = target
                .get("folderSuffix")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let package_suffix = target
                .get("packageSuffix")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            serde_json::json!({
                "target": target_name,
                "binary": target.get("binary").cloned().unwrap_or(serde_json::Value::Null),
                "folder": format!("wrapper-js/platform-packages/ossplate-{}", folder_suffix),
                "packageName": format!("{}-{}", config.packages.npm_package, package_suffix),
            })
        })
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "paths": {
            "jsWrapperLauncher": js_launcher,
            "pythonModule": python_module,
            "pythonPackageDir": python_package_dir,
            "pythonEntrypoint": format!("{}.cli:main", python_module_name(&config.packages.python_package)),
            "pythonCliModulePath": source_root,
            "scaffoldMirrors": [
                "wrapper-js/scaffold",
                format!("wrapper-py/src/{}/scaffold", python_module_name(&config.packages.python_package))
            ]
        },
        "runtimePackages": runtime_packages
    }))
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

fn read_json(root: &Path, relative_path: &str) -> Result<serde_json::Value> {
    serde_json::from_str(
        &fs::read_to_string(root.join(relative_path))
            .with_context(|| format!("failed to read {}", root.join(relative_path).display()))?,
    )
    .with_context(|| format!("failed to parse {}", root.join(relative_path).display()))
}

fn read_optional_json(root: &Path, relative_path: &str) -> Result<Option<serde_json::Value>> {
    let target = root.join(relative_path);
    if !target.is_file() {
        return Ok(None);
    }
    Ok(Some(read_json(root, relative_path)?))
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
            path: ".github/workflows/live-e2e-published.yml",
            validate: text::validate_live_e2e_workflow,
            sync: text::sync_live_e2e_workflow,
        },
        ManagedFile {
            path: "core-rs/Cargo.toml",
            validate: metadata::validate_cargo_toml,
            sync: metadata::sync_cargo_toml,
        },
        ManagedFile {
            path: "runtime-targets.json",
            validate: metadata::validate_runtime_targets_json,
            sync: metadata::sync_runtime_targets_json,
        },
        ManagedFile {
            path: "core-rs/runtime-targets.json",
            validate: metadata::validate_core_runtime_targets_json,
            sync: metadata::sync_runtime_targets_json,
        },
        ManagedFile {
            path: "scaffold-payload.json",
            validate: metadata::validate_scaffold_payload_json,
            sync: metadata::sync_scaffold_payload_json,
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
