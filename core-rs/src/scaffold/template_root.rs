use crate::embedded_template::materialize_embedded_template_root;
use crate::source_checkout::ensure_source_checkout;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

pub(crate) fn discover_template_root() -> Result<PathBuf> {
    if let Some(explicit) = std::env::var_os("OSSPLATE_TEMPLATE_ROOT") {
        let explicit = PathBuf::from(explicit);
        if explicit.join("ossplate.toml").is_file() {
            return Ok(explicit);
        }
        bail!("OSSPLATE_TEMPLATE_ROOT does not point to a scaffold root containing ossplate.toml");
    }

    let exe = std::env::current_exe().context("failed to determine current executable path")?;
    for ancestor in exe.ancestors() {
        if ancestor.join("ossplate.toml").is_file() {
            return Ok(ancestor.to_path_buf());
        }
    }
    std::env::current_dir()
        .context("failed to determine current directory")?
        .ancestors()
        .find(|ancestor| ancestor.join("ossplate.toml").is_file())
        .map(Path::to_path_buf)
        .map_or_else(materialize_embedded_template_root, Ok)
}

pub(crate) fn ensure_scaffold_source_root(root: &Path) -> Result<()> {
    ensure_source_checkout(root, "create/init require")
}
