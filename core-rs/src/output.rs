use crate::config::ToolConfig;
use crate::release::PublishRegistry;
use crate::sync::{SyncChangePlan, ValidationOutput};
use crate::upgrade::Compatibility;
use crate::verify::VerifyStepResult;
use anyhow::{bail, Result};
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

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

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SyncOutput {
    pub(crate) ok: bool,
    pub(crate) mode: &'static str,
    pub(crate) issues: Vec<crate::sync::ValidationIssue>,
    pub(crate) changes: Vec<SyncChangeOutput>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SyncChangeOutput {
    pub(crate) file: String,
    pub(crate) changed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) synced: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct BootstrapOutput {
    pub(crate) ok: bool,
    pub(crate) action: &'static str,
    pub(crate) path: String,
    pub(crate) config: ToolConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) created: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) initialized: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct InspectOutput {
    pub(crate) config: ToolConfig,
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
    #[serde(rename = "managedFiles")]
    pub(crate) managed_files: Vec<String>,
    #[serde(rename = "runtimeTargets")]
    pub(crate) runtime_targets: Value,
    #[serde(rename = "scaffoldPayload")]
    pub(crate) scaffold_payload: Value,
    #[serde(rename = "sourceCheckout")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) source_checkout: Option<Value>,
    #[serde(rename = "derived")]
    pub(crate) derived: Value,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PublishPlanOutput {
    pub(crate) ok: bool,
    pub(crate) root: String,
    pub(crate) registry: PublishRegistry,
    #[serde(rename = "dryRun")]
    pub(crate) dry_run: bool,
    #[serde(rename = "skipExisting")]
    pub(crate) skip_existing: bool,
    pub(crate) helper: String,
    pub(crate) argv: Vec<String>,
    #[serde(rename = "selectedRegistries")]
    pub(crate) selected_registries: Vec<String>,
    pub(crate) host: Value,
    pub(crate) preflight: Value,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct VerifyOutput {
    pub(crate) ok: bool,
    pub(crate) steps: Vec<VerifyStepResult>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct UpgradeOutput {
    pub(crate) ok: bool,
    pub(crate) apply: bool,
    #[serde(rename = "fromVersion")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) from_version: Option<u64>,
    #[serde(rename = "toVersion")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) to_version: Option<u64>,
    pub(crate) compatibility: Compatibility,
    #[serde(rename = "recommendedAction")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) recommended_action: Option<String>,
    #[serde(rename = "upgradePath")]
    pub(crate) upgrade_path: Vec<String>,
    #[serde(rename = "blockingReason")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) blocking_reason: Option<String>,
    #[serde(rename = "changedFiles")]
    pub(crate) changed_files: Vec<String>,
    #[serde(rename = "canApply")]
    pub(crate) can_apply: bool,
    #[serde(rename = "stepPlans")]
    pub(crate) step_plans: Vec<crate::upgrade::StepPlan>,
}

pub(crate) fn render_sync_output(
    mode: &'static str,
    issues: Vec<crate::sync::ValidationIssue>,
    changes: Vec<SyncChangePlan>,
    include_synced: bool,
) -> Result<String> {
    let output = SyncOutput {
        ok: issues.is_empty(),
        mode,
        issues,
        changes: changes
            .into_iter()
            .map(|change| SyncChangeOutput {
                file: change.path,
                changed: true,
                synced: include_synced.then_some(change.synced),
            })
            .collect(),
    };
    Ok(serde_json::to_string(&output)?)
}

pub(crate) fn render_bootstrap_output(
    action: &'static str,
    path: &Path,
    config: ToolConfig,
) -> Result<String> {
    let output = BootstrapOutput {
        ok: true,
        action,
        path: path.display().to_string(),
        config,
        created: (action == "create").then_some(true),
        initialized: (action == "init").then_some(true),
    };
    Ok(serde_json::to_string(&output)?)
}

pub(crate) fn render_inspect_output(output: InspectOutput) -> Result<String> {
    Ok(serde_json::to_string(&output)?)
}

pub(crate) fn render_publish_plan_output(output: PublishPlanOutput) -> Result<String> {
    Ok(serde_json::to_string(&output)?)
}

pub(crate) fn render_verify_output(steps: Vec<VerifyStepResult>) -> Result<String> {
    let ok = steps.iter().all(|step| step.ok || step.skipped);
    Ok(serde_json::to_string(&VerifyOutput { ok, steps })?)
}

pub(crate) fn render_upgrade_output(output: UpgradeOutput) -> Result<String> {
    Ok(serde_json::to_string(&output)?)
}

pub(crate) fn print_validation_output(output: &ValidationOutput, as_json: bool) -> Result<()> {
    if as_json {
        println!("{}", serde_json::to_string(output)?);
    } else if output.ok {
        println!("validation ok");
        if !output.warnings.is_empty() {
            println!("warnings:");
            for warning in &output.warnings {
                println!("- {warning}");
            }
        }
    } else {
        println!(
            "{}",
            crate::sync::format_human_issues("validation failed:", &output.issues)
        );
        if !output.warnings.is_empty() {
            println!("warnings:");
            for warning in &output.warnings {
                println!("- {warning}");
            }
        }
    }

    if output.ok {
        Ok(())
    } else {
        bail!("validation failed")
    }
}
