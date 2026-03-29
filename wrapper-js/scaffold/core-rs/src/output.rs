use crate::sync::ValidationOutput;
use anyhow::{bail, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct VersionOutput {
    pub(crate) tool: &'static str,
    pub(crate) version: &'static str,
}

pub(crate) fn render_version_output() -> Result<String> {
    Ok(serde_json::to_string(&VersionOutput {
        tool: env!("CARGO_BIN_NAME"),
        version: env!("CARGO_PKG_VERSION"),
    })?)
}

pub(crate) fn print_validation_output(output: &ValidationOutput, as_json: bool) -> Result<()> {
    if as_json {
        println!("{}", serde_json::to_string(output)?);
    } else if output.ok {
        println!("validation ok");
    } else {
        println!(
            "{}",
            crate::sync::format_human_issues("validation failed:", &output.issues)
        );
    }

    if output.ok {
        Ok(())
    } else {
        bail!("validation failed")
    }
}
