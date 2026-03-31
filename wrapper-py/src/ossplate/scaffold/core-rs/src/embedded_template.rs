use anyhow::{Context, Result};
use std::fs;
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

include!(concat!(env!("OUT_DIR"), "/embedded_template.rs"));

pub(crate) fn materialize_embedded_template_root() -> Result<PathBuf> {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("failed to derive embedded template timestamp")?
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ossplate-embedded-template-{}-{unique}",
        std::process::id()
    ));

    for (relative_path, contents) in EMBEDDED_TEMPLATE_FILES {
        let destination = root.join(relative_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(&destination, contents)
            .with_context(|| format!("failed to write {}", destination.display()))?;
    }

    Ok(root)
}

#[cfg(test)]
pub(crate) fn embedded_template_contains(root: &Path, relative_path: &str) -> bool {
    root.join(relative_path).is_file()
}
