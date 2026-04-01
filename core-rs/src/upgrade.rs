use crate::config::{latest_scaffold_version, load_config};
use crate::embedded_template::materialize_embedded_template_root;
use crate::output::{render_upgrade_output, UpgradeOutput};
use crate::scaffold_manifest::required_source_paths;
use crate::upgrade_catalog::{authored_versions, MigrationDefinition, VersionSpec};
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[allow(dead_code)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Compatibility {
    Current,
    UpgradeSupported,
    UpgradeRequiresManualSteps,
    RecreateRecommended,
    Unsupported,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct StepPlan {
    pub(crate) step: String,
    #[serde(rename = "fromVersion")]
    pub(crate) from_version: u64,
    #[serde(rename = "toVersion")]
    pub(crate) to_version: u64,
    #[serde(rename = "changedFiles")]
    pub(crate) changed_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CompatibilityReport {
    #[serde(rename = "scaffoldVersion")]
    pub(crate) scaffold_version: Option<u64>,
    #[serde(rename = "latestScaffoldVersion")]
    pub(crate) latest_scaffold_version: u64,
    pub(crate) compatibility: Compatibility,
    #[serde(rename = "recommendedAction")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) recommended_action: Option<String>,
    #[serde(rename = "upgradePath")]
    pub(crate) upgrade_path: Vec<String>,
    #[serde(rename = "blockingReason")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) blocking_reason: Option<String>,
    #[serde(rename = "missingPaths")]
    pub(crate) missing_paths: Vec<String>,
}

pub(crate) fn inspect_compatibility(root: &Path) -> Result<CompatibilityReport> {
    let config = load_config(root)?;
    Ok(detect_compatibility(root, &config))
}

pub(crate) fn upgrade_plan_json(root: &Path) -> Result<String> {
    render_upgrade(root, false)
}

pub(crate) fn upgrade_apply_json(root: &Path) -> Result<String> {
    render_upgrade(root, true)
}

fn render_upgrade(root: &Path, apply: bool) -> Result<String> {
    let report = inspect_compatibility(root)?;

    match report.compatibility {
        Compatibility::Current => {
            return render_upgrade_output(UpgradeOutput {
                ok: true,
                apply,
                from_version: report.scaffold_version,
                to_version: Some(report.latest_scaffold_version),
                compatibility: report.compatibility,
                recommended_action: report.recommended_action,
                upgrade_path: report.upgrade_path,
                blocking_reason: report.blocking_reason,
                changed_files: Vec::new(),
                manual_follow_ups: Vec::new(),
                can_apply: true,
                step_plans: Vec::new(),
            });
        }
        Compatibility::UpgradeSupported => {}
        Compatibility::UpgradeRequiresManualSteps => {
            bail!(
                "upgrade requires manual steps before apply; run `ossplate upgrade --plan --json`"
            )
        }
        Compatibility::RecreateRecommended => {
            let reason = report
                .blocking_reason
                .unwrap_or_else(|| "upgrade path is unavailable".to_string());
            bail!("upgrade is not supported for this descendant; recreate is recommended: {reason}")
        }
        Compatibility::Unsupported => {
            let reason = report
                .blocking_reason
                .unwrap_or_else(|| "repo shape is unsupported".to_string());
            bail!("upgrade is not supported for this repo shape: {reason}")
        }
    }

    let from_version = report
        .scaffold_version
        .context("upgradeable descendant must resolve a starting scaffold version")?;
    let steps = resolve_upgrade_path(from_version, report.latest_scaffold_version)
        .context("upgrade path should exist for upgrade-supported descendant")?;

    let source_root = discover_upgrade_source_root(root)?;
    let working_root = if apply {
        root.to_path_buf()
    } else {
        clone_tree_to_temp(root)?
    };
    let (step_plans, changed_files) = execute_upgrade_steps(&source_root, &working_root, &steps)?;

    render_upgrade_output(UpgradeOutput {
        ok: true,
        apply,
        from_version: Some(from_version),
        to_version: Some(report.latest_scaffold_version),
        compatibility: Compatibility::UpgradeSupported,
        recommended_action: Some("upgrade".to_string()),
        upgrade_path: steps.iter().map(MigrationDefinition::label).collect(),
        blocking_reason: None,
        changed_files,
        manual_follow_ups: Vec::new(),
        can_apply: true,
        step_plans,
    })
}

fn detect_compatibility(root: &Path, config: &crate::config::ToolConfig) -> CompatibilityReport {
    let latest = latest_scaffold_version();
    let current_missing = missing_current_source_paths(root);

    if config.template.is_canonical {
        let compatibility = if config.template.scaffold_version == Some(latest)
            && spec_for_version(latest).is_some_and(|spec| spec.fingerprint.matches(root))
        {
            Compatibility::Current
        } else {
            Compatibility::Unsupported
        };
        return CompatibilityReport {
            scaffold_version: config.template.scaffold_version,
            latest_scaffold_version: latest,
            compatibility: compatibility.clone(),
            recommended_action: (compatibility == Compatibility::Unsupported)
                .then_some("stop".to_string()),
            upgrade_path: Vec::new(),
            blocking_reason: (compatibility == Compatibility::Unsupported).then_some(
                "canonical scaffold checkout must match the latest scaffold fingerprint"
                    .to_string(),
            ),
            missing_paths: current_missing,
        };
    }

    let detected = detect_descendant_version(root, config);
    let compatibility = match detected.version {
        Some(version) if !detected.exact_match && version > latest => Compatibility::Unsupported,
        Some(version) if !detected.exact_match && known_versions().contains(&version) => {
            Compatibility::Unsupported
        }
        Some(_) if !detected.exact_match => Compatibility::RecreateRecommended,
        Some(version) if version == latest => Compatibility::Current,
        Some(version) if version > latest => Compatibility::Unsupported,
        Some(version) => {
            if resolve_upgrade_path(version, latest).is_some() {
                Compatibility::UpgradeSupported
            } else {
                Compatibility::RecreateRecommended
            }
        }
        None => {
            if current_missing.is_empty() {
                Compatibility::Current
            } else {
                Compatibility::RecreateRecommended
            }
        }
    };

    let resolved_upgrade_path = if detected.exact_match {
        detected
            .version
            .and_then(|version| resolve_upgrade_path(version, latest))
    } else {
        None
    };
    let upgrade_path = resolved_upgrade_path
        .as_ref()
        .map(|steps| steps.iter().map(MigrationDefinition::label).collect())
        .unwrap_or_default();

    let blocking_reason = detected.blocking_reason.or_else(|| match compatibility {
        Compatibility::RecreateRecommended => detected.version.map(|version| {
            format!("upgrade path is unavailable from scaffold version {version} to {latest}")
        }),
        Compatibility::Unsupported if detected.version.is_some() => {
            detected.version.map(|version| {
                format!("repo cannot be upgraded safely from declared scaffold version {version}")
            })
        }
        _ => None,
    });

    let recommended_action = match compatibility {
        Compatibility::Current => None,
        Compatibility::UpgradeSupported | Compatibility::UpgradeRequiresManualSteps => {
            Some("upgrade".to_string())
        }
        Compatibility::RecreateRecommended => Some("recreate".to_string()),
        Compatibility::Unsupported => Some("stop".to_string()),
    };

    CompatibilityReport {
        scaffold_version: detected.version,
        latest_scaffold_version: latest,
        compatibility,
        recommended_action,
        upgrade_path,
        blocking_reason,
        missing_paths: current_missing,
    }
}

#[derive(Debug)]
struct DetectedDescendantVersion {
    version: Option<u64>,
    exact_match: bool,
    blocking_reason: Option<String>,
}

fn detect_descendant_version(
    root: &Path,
    config: &crate::config::ToolConfig,
) -> DetectedDescendantVersion {
    match config.template.scaffold_version {
        Some(version) => {
            if !known_versions().contains(&version) {
                let reason = if version > latest_scaffold_version() {
                    format!(
                        "repo declares scaffold version {version}, which is newer than this ossplate checkout supports"
                    )
                } else {
                    format!(
                        "upgrade path is unavailable from scaffold version {version} to {}",
                        latest_scaffold_version()
                    )
                };
                return DetectedDescendantVersion {
                    version: Some(version),
                    exact_match: false,
                    blocking_reason: Some(reason),
                };
            }

            let fingerprint = fingerprint_for_version(version);
            if fingerprint.matches(root) {
                return DetectedDescendantVersion {
                    version: Some(version),
                    exact_match: true,
                    blocking_reason: None,
                };
            }

            let reason = if version > latest_scaffold_version() {
                format!(
                    "repo declares scaffold version {version}, which is newer than this ossplate checkout supports"
                )
            } else {
                format!(
                    "repo declares scaffold version {version}, but its managed scaffold fingerprint does not match that version"
                )
            };
            DetectedDescendantVersion {
                version: Some(version),
                exact_match: false,
                blocking_reason: Some(reason),
            }
        }
        None => {
            for version in known_versions() {
                let fingerprint = fingerprint_for_version(version);
                if fingerprint.matches(root) {
                    return DetectedDescendantVersion {
                        version: Some(version),
                        exact_match: true,
                        blocking_reason: None,
                    };
                }
            }

            DetectedDescendantVersion {
                version: None,
                exact_match: false,
                blocking_reason: Some(
                    "repo is unversioned and does not exactly match any known scaffold fingerprint"
                        .to_string(),
                ),
            }
        }
    }
}

fn execute_upgrade_steps(
    source_root: &Path,
    working_root: &Path,
    steps: &[MigrationDefinition],
) -> Result<(Vec<StepPlan>, Vec<String>)> {
    let mut step_plans = Vec::new();
    let mut all_changed = BTreeSet::new();

    for step in steps {
        let before = snapshot_tree(working_root)?;
        (step.apply)(source_root, working_root)?;
        let after = snapshot_tree(working_root)?;
        let changed_files = diff_snapshots(&before, &after);
        all_changed.extend(changed_files.iter().cloned());
        step_plans.push(StepPlan {
            step: step.label(),
            from_version: step.from_version,
            to_version: step.to_version,
            changed_files,
        });
    }

    Ok((step_plans, all_changed.into_iter().collect()))
}

fn resolve_upgrade_path(
    from_version: u64,
    latest_version: u64,
) -> Option<Vec<MigrationDefinition>> {
    if from_version == latest_version {
        return Some(Vec::new());
    }

    let registry = migration_registry();
    let mut current = from_version;
    let mut path = Vec::new();

    while current < latest_version {
        let step = registry.iter().find(|step| step.from_version == current)?;
        path.push(step.clone());
        current = step.to_version;
    }

    (current == latest_version).then_some(path)
}

fn migration_registry() -> Vec<MigrationDefinition> {
    authored_versions()
        .into_iter()
        .filter_map(|spec| spec.migration_from_previous)
        .collect()
}

fn spec_for_version(version: u64) -> Option<VersionSpec> {
    authored_versions()
        .into_iter()
        .find(|spec| spec.version == version)
}

fn fingerprint_for_version(version: u64) -> crate::upgrade_catalog::VersionFingerprint {
    spec_for_version(version)
        .map(|spec| spec.fingerprint)
        .unwrap_or(crate::upgrade_catalog::VersionFingerprint {
            required_paths: Vec::new(),
            forbidden_paths: Vec::new(),
        })
}

fn known_versions() -> Vec<u64> {
    let mut versions = authored_versions()
        .into_iter()
        .map(|spec| spec.version)
        .collect::<Vec<_>>();
    versions.sort();
    versions.dedup();
    versions.reverse();
    versions
}

fn missing_current_source_paths(root: &Path) -> Vec<String> {
    required_source_paths()
        .into_iter()
        .filter(|path| !root.join(path).exists())
        .collect()
}

fn discover_upgrade_source_root(target_root: &Path) -> Result<PathBuf> {
    let target_root = target_root.canonicalize()?;
    let source_root = crate::scaffold::discover_template_root()?;
    let source_root = source_root.canonicalize()?;

    if source_root != target_root {
        crate::scaffold::ensure_scaffold_source_root(&source_root)?;
        return Ok(source_root);
    }

    let config = load_config(&target_root)?;
    if config.template.is_canonical {
        crate::scaffold::ensure_scaffold_source_root(&target_root)?;
        return Ok(target_root);
    }

    materialize_embedded_template_root()
}

fn clone_tree_to_temp(source_root: &Path) -> Result<PathBuf> {
    let temp_root = std::env::temp_dir().join(format!("ossplate-upgrade-plan-{}", unique_suffix()));
    copy_tree(source_root, &temp_root)?;
    Ok(temp_root)
}

fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time must advance")
        .as_nanos()
}

fn snapshot_tree(root: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut entries = BTreeMap::new();
    collect_snapshot(root, root, &mut entries)?;
    Ok(entries)
}

fn collect_snapshot(
    root: &Path,
    current: &Path,
    entries: &mut BTreeMap<String, Vec<u8>>,
) -> Result<()> {
    for entry in
        fs::read_dir(current).with_context(|| format!("failed to read {}", current.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .to_string();
        if should_skip_snapshot(&relative) {
            continue;
        }
        if entry.file_type()?.is_dir() {
            collect_snapshot(root, &path, entries)?;
        } else if entry.file_type()?.is_file() {
            entries.insert(relative, fs::read(&path)?);
        }
    }
    Ok(())
}

fn should_skip_snapshot(relative_path: &str) -> bool {
    matches!(
        relative_path.split('/').next(),
        Some(".git" | "target" | "node_modules" | ".venv" | "venv" | "__pycache__")
    )
}

fn diff_snapshots(
    before: &BTreeMap<String, Vec<u8>>,
    after: &BTreeMap<String, Vec<u8>>,
) -> Vec<String> {
    let keys = before
        .keys()
        .chain(after.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    keys.into_iter()
        .filter(|path| before.get(path) != after.get(path))
        .collect()
}

fn copy_tree(source_root: &Path, target_root: &Path) -> Result<()> {
    if target_root.exists() {
        fs::remove_dir_all(target_root)
            .with_context(|| format!("failed to remove {}", target_root.display()))?;
    }
    fs::create_dir_all(target_root)
        .with_context(|| format!("failed to create {}", target_root.display()))?;
    copy_tree_recursive(source_root, source_root, target_root)
}

fn copy_tree_recursive(source_root: &Path, current: &Path, target_root: &Path) -> Result<()> {
    for entry in
        fs::read_dir(current).with_context(|| format!("failed to read {}", current.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(source_root).unwrap();
        if should_skip_snapshot(&relative.to_string_lossy()) {
            continue;
        }
        let target_path = target_root.join(relative);
        if entry.file_type()?.is_dir() {
            fs::create_dir_all(&target_path)
                .with_context(|| format!("failed to create {}", target_path.display()))?;
            copy_tree_recursive(source_root, &path, target_root)?;
        } else if entry.file_type()?.is_file() {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }
            fs::copy(&path, &target_path).with_context(|| {
                format!(
                    "failed to copy {} -> {}",
                    path.display(),
                    target_path.display()
                )
            })?;
        }
    }
    Ok(())
}
