use anyhow::Result;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::config::{load_config, write_config};
use crate::scaffold::refresh_embedded_template_root;
use crate::scaffold_manifest::{
    current_core_source_checkout_manifest, current_repo_source_checkout_manifest,
    current_scaffold_payload_manifest, write_path_manifest, PathManifest,
};

#[derive(Debug, Clone)]
pub(crate) struct VersionSpec {
    pub(crate) version: u64,
    pub(crate) fingerprint: VersionFingerprint,
    pub(crate) migration_from_previous: Option<MigrationDefinition>,
}

#[derive(Debug, Clone)]
pub(crate) struct MigrationDefinition {
    pub(crate) from_version: u64,
    pub(crate) to_version: u64,
    pub(crate) apply: fn(&Path, &Path) -> Result<()>,
    pub(crate) planned_changes: fn() -> Vec<String>,
}

impl MigrationDefinition {
    pub(crate) fn label(&self) -> String {
        format!("{}->{}", self.from_version, self.to_version)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct VersionFingerprint {
    pub(crate) required_paths: Vec<String>,
    pub(crate) forbidden_paths: Vec<String>,
    pub(crate) exact_json_files: Vec<JsonFingerprint>,
}

#[derive(Debug, Clone)]
pub(crate) struct JsonFingerprint {
    pub(crate) path: String,
    pub(crate) expected: Value,
}

impl VersionFingerprint {
    pub(crate) fn matches(&self, root: &Path) -> bool {
        self.required_paths
            .iter()
            .all(|path| root.join(path).exists())
            && self
                .forbidden_paths
                .iter()
                .all(|path| !root.join(path).exists())
            && self.exact_json_files.iter().all(|fingerprint| {
                let Ok(contents) = fs::read_to_string(root.join(&fingerprint.path)) else {
                    return false;
                };
                let Ok(actual) = serde_json::from_str::<Value>(&contents) else {
                    return false;
                };
                actual == fingerprint.expected
            })
    }
}

const VERSION_1_REMOVED_PATHS: &[&str] = &[
    "core-rs/build.rs",
    "core-rs/src/embedded_template.rs",
    "core-rs/src/upgrade.rs",
    "core-rs/src/upgrade_catalog.rs",
    "core-rs/src/verify.rs",
    "scripts/stage-embedded-template.mjs",
    "scripts/package-js.mjs",
];

const VERSION_2_REMOVED_PATHS: &[&str] = &["core-rs/src/upgrade_catalog.rs"];

pub(crate) fn authored_versions() -> Vec<VersionSpec> {
    vec![
        VersionSpec {
            version: 1,
            fingerprint: fingerprint_for_historical_version(1),
            migration_from_previous: None,
        },
        VersionSpec {
            version: 2,
            fingerprint: fingerprint_for_historical_version(2),
            migration_from_previous: Some(MigrationDefinition {
                from_version: 1,
                to_version: 2,
                apply: apply_v1_to_v2,
                planned_changes: planned_changes_for_v2,
            }),
        },
        VersionSpec {
            version: 3,
            fingerprint: fingerprint_for_current_version(),
            migration_from_previous: Some(MigrationDefinition {
                from_version: 2,
                to_version: 3,
                apply: apply_v2_to_v3,
                planned_changes: planned_changes_for_v3,
            }),
        },
    ]
}

#[cfg(test)]
pub(crate) fn latest_authored_version() -> u64 {
    authored_versions()
        .into_iter()
        .map(|spec| spec.version)
        .max()
        .expect("at least one authored scaffold version must exist")
}

fn fingerprint_for_current_version() -> VersionFingerprint {
    VersionFingerprint {
        required_paths: current_core_source_checkout_manifest().required_paths,
        forbidden_paths: Vec::new(),
        exact_json_files: vec![
            json_fingerprint(
                "source-checkout.json",
                manifest_to_json(&current_repo_source_checkout_manifest()),
            ),
            json_fingerprint(
                "scaffold-payload.json",
                manifest_to_json(&current_scaffold_payload_manifest()),
            ),
            json_fingerprint(
                "core-rs/source-checkout.json",
                manifest_to_json(&current_core_source_checkout_manifest()),
            ),
        ],
    }
}

fn fingerprint_for_historical_version(version: u64) -> VersionFingerprint {
    let removed = removed_paths_for_version(version);
    VersionFingerprint {
        required_paths: current_core_source_checkout_manifest()
            .required_paths
            .into_iter()
            .filter(|path| !removed.contains(path))
            .collect(),
        forbidden_paths: removed.iter().cloned().collect(),
        exact_json_files: vec![
            json_fingerprint(
                "source-checkout.json",
                manifest_to_json(&repo_source_checkout_for_version(version)),
            ),
            json_fingerprint(
                "scaffold-payload.json",
                manifest_to_json(&scaffold_payload_for_version(version)),
            ),
            json_fingerprint(
                "core-rs/source-checkout.json",
                manifest_to_json(&core_source_checkout_for_version(version)),
            ),
        ],
    }
}

fn planned_changes_for_v2() -> Vec<String> {
    owned_descendant_paths_for_version(2)
}

fn planned_changes_for_v3() -> Vec<String> {
    owned_descendant_paths_for_version(3)
}

fn apply_v1_to_v2(source_root: &Path, target_root: &Path) -> Result<()> {
    apply_version_owned_changes(source_root, target_root, 2)
}

fn apply_v2_to_v3(source_root: &Path, target_root: &Path) -> Result<()> {
    apply_version_owned_changes(source_root, target_root, 3)
}

fn apply_version_owned_changes(source_root: &Path, target_root: &Path, version: u64) -> Result<()> {
    for relative_path in owned_descendant_paths_for_version(version) {
        let source_path = source_root.join(&relative_path);
        if !source_path.exists() {
            continue;
        }
        let target_path = target_root.join(&relative_path);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&source_path, &target_path)?;
    }

    write_path_manifest(
        &target_root.join("source-checkout.json"),
        &repo_source_checkout_for_version(version),
    )?;
    write_path_manifest(
        &target_root.join("scaffold-payload.json"),
        &scaffold_payload_for_version(version),
    )?;
    write_path_manifest(
        &target_root.join("core-rs/source-checkout.json"),
        &core_source_checkout_for_version(version),
    )?;

    let mut config = load_config(target_root)?;
    config.template.scaffold_version = Some(version);
    write_config(target_root, &config)?;
    refresh_embedded_template_root(target_root)
}

fn owned_descendant_paths_for_version(version: u64) -> Vec<String> {
    let mut paths = BTreeSet::new();
    paths.insert("ossplate.toml".to_string());
    paths.insert("scaffold-payload.json".to_string());
    paths.insert("source-checkout.json".to_string());

    let scaffold_payload = scaffold_payload_for_version(version);
    let template_only = scaffold_payload
        .template_only_paths
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();

    for relative_path in scaffold_payload.required_paths {
        if !template_only.contains(&relative_path) {
            paths.insert(relative_path);
        }
    }

    paths.into_iter().collect()
}

fn repo_source_checkout_for_version(version: u64) -> PathManifest {
    let mut manifest = current_repo_source_checkout_manifest();
    manifest
        .required_paths
        .retain(|path| !removed_paths_for_version(version).contains(path));
    manifest
}

fn scaffold_payload_for_version(version: u64) -> PathManifest {
    let mut manifest = current_scaffold_payload_manifest();
    let template_only_paths = manifest.template_only_paths.clone();
    manifest.required_paths.retain(|path| {
        !removed_paths_for_version(version).contains(path) && !template_only_paths.contains(path)
    });
    manifest
}

fn core_source_checkout_for_version(version: u64) -> PathManifest {
    let mut manifest = current_core_source_checkout_manifest();
    manifest
        .required_paths
        .retain(|path| !removed_paths_for_version(version).contains(path));
    manifest
}

fn removed_paths_for_version(version: u64) -> BTreeSet<String> {
    match version {
        1 => VERSION_1_REMOVED_PATHS
            .iter()
            .map(|path| path.to_string())
            .collect(),
        2 => VERSION_2_REMOVED_PATHS
            .iter()
            .map(|path| path.to_string())
            .collect(),
        3 => BTreeSet::new(),
        _ => BTreeSet::new(),
    }
}

fn manifest_to_json(manifest: &PathManifest) -> Value {
    serde_json::to_value(manifest).expect("path manifest must serialize")
}

fn json_fingerprint(path: &str, expected: Value) -> JsonFingerprint {
    JsonFingerprint {
        path: path.to_string(),
        expected,
    }
}
