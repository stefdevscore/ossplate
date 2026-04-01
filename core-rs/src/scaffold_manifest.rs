use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct PathManifest {
    #[serde(rename = "requiredPaths")]
    pub(crate) required_paths: Vec<String>,
    #[serde(
        rename = "templateOnlyPaths",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub(crate) template_only_paths: Vec<String>,
    #[serde(
        rename = "excludedPrefixes",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub(crate) excluded_prefixes: Vec<String>,
}

pub(crate) fn current_core_source_checkout_manifest() -> PathManifest {
    parse_manifest_str(
        include_str!("../source-checkout.json"),
        "core-rs/source-checkout.json",
    )
    .expect("core-rs/source-checkout.json must parse")
}

pub(crate) fn current_repo_source_checkout_manifest() -> PathManifest {
    parse_manifest_str(
        include_str!("../source-checkout.json"),
        "source-checkout.json",
    )
    .expect("source-checkout.json must parse")
}

pub(crate) fn current_scaffold_payload_manifest() -> PathManifest {
    parse_manifest_str(
        include_str!("../scaffold-payload.json"),
        "scaffold-payload.json",
    )
    .expect("scaffold-payload.json must parse")
}

pub(crate) fn required_source_paths() -> Vec<String> {
    current_core_source_checkout_manifest().required_paths
}

pub(crate) fn read_path_manifest(path: &Path) -> Result<PathManifest> {
    let rendered =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    parse_manifest_str(&rendered, &path.display().to_string())
}

pub(crate) fn write_path_manifest(path: &Path, manifest: &PathManifest) -> Result<()> {
    let mut rendered =
        serde_json::to_string_pretty(manifest).context("failed to serialize path manifest")?;
    rendered.push('\n');
    fs::write(path, rendered).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

pub(crate) fn template_only_paths_from_root(root: &Path) -> Result<HashSet<String>> {
    let scaffold_payload = read_path_manifest(&root.join("scaffold-payload.json"))?;
    let source_checkout = read_path_manifest(&root.join("source-checkout.json"))?;
    let scaffold_paths = scaffold_payload
        .template_only_paths
        .into_iter()
        .collect::<HashSet<_>>();
    let source_paths = source_checkout
        .template_only_paths
        .into_iter()
        .collect::<HashSet<_>>();
    if scaffold_paths != source_paths {
        bail!(
            "templateOnlyPaths must match between scaffold-payload.json and source-checkout.json"
        );
    }
    Ok(scaffold_paths)
}

fn parse_manifest_str(contents: &str, label: &str) -> Result<PathManifest> {
    serde_json::from_str(contents).with_context(|| format!("failed to parse {label}"))
}
