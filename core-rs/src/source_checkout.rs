use anyhow::{bail, Result};
use std::path::Path;

pub(crate) fn ensure_source_checkout(root: &Path, requirement_prefix: &str) -> Result<()> {
    let missing = missing_source_paths(root);
    if missing.is_empty() {
        return Ok(());
    }

    if let Ok(report) = crate::upgrade::inspect_compatibility(root) {
        match report.compatibility {
            crate::upgrade::Compatibility::UpgradeSupported
            | crate::upgrade::Compatibility::UpgradeRequiresManualSteps => {
                let path = if report.upgrade_path.is_empty() {
                    "unknown upgrade path".to_string()
                } else {
                    report.upgrade_path.join(", ")
                };
                bail!(
                    "{requirement_prefix} a full scaffold source checkout for scaffold version {}; this repo is on scaffold version {} and should use `ossplate upgrade --plan --json` or `ossplate upgrade --json` (path: {path})",
                    report.latest_scaffold_version,
                    report.scaffold_version.unwrap_or_default(),
                )
            }
            crate::upgrade::Compatibility::RecreateRecommended => {
                let reason = report
                    .blocking_reason
                    .unwrap_or_else(|| "upgrade path is unavailable".to_string());
                bail!(
                    "{requirement_prefix} a full scaffold source checkout for scaffold version {}; this repo should be recreated or manually resynced before reuse as a scaffold source: {reason}",
                    report.latest_scaffold_version,
                )
            }
            crate::upgrade::Compatibility::Unsupported => {
                let reason = report.blocking_reason.unwrap_or_else(|| {
                    "repo compatibility is unsupported for scaffold reuse".to_string()
                });
                bail!("{requirement_prefix} a full scaffold source checkout; {reason}")
            }
            crate::upgrade::Compatibility::Current => {}
        }
    }

    bail!(
        "{requirement_prefix} a full scaffold source checkout; missing required scaffold paths: {}",
        missing.join(", ")
    )
}

fn missing_source_paths(root: &Path) -> Vec<String> {
    crate::scaffold_manifest::required_source_paths()
        .iter()
        .filter(|path| !root.join(path).exists())
        .cloned()
        .collect()
}
