use crate::scaffold_manifest::required_source_paths;
use anyhow::{bail, Result};
use std::path::Path;

pub(crate) fn ensure_source_checkout(root: &Path, requirement_prefix: &str) -> Result<()> {
    let missing = missing_source_paths(root);
    if missing.is_empty() {
        return Ok(());
    }

    bail!(
        "{requirement_prefix} a full scaffold source checkout; missing required scaffold paths: {}",
        missing.join(", ")
    )
}

fn missing_source_paths(root: &Path) -> Vec<String> {
    required_source_paths()
        .iter()
        .filter(|path| !root.join(path).exists())
        .cloned()
        .collect()
}
