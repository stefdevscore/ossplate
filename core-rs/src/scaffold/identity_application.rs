use crate::config::{
    generated_project_description, latest_scaffold_version, load_config, write_config,
    IdentityOverrides, ToolConfig, GENERATED_AUTHOR_EMAIL_PLACEHOLDER,
    GENERATED_AUTHOR_NAME_PLACEHOLDER, GENERATED_REPOSITORY_PLACEHOLDER,
};
use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn apply_config_overrides_to_target(
    target_root: &Path,
    source_root: &Path,
    overrides: &IdentityOverrides,
    action: &str,
) -> Result<()> {
    let source_config = load_config(source_root)?;
    let mut config = if target_root.join("ossplate.toml").is_file() {
        load_config(target_root)?
    } else {
        source_config.clone()
    };
    let original = config.clone();

    apply_overrides(&mut config, overrides);
    apply_generated_identity_defaults(&mut config, &source_config, overrides);
    apply_template_mode(&mut config, action, source_root, target_root)?;
    relocate_generated_identity_paths(target_root, &original, &config)?;
    remove_generated_python_runtime_dirs(target_root, &original, &config)?;
    normalize_cargo_lock_identity(target_root, &original, &config)?;
    normalize_package_lock_identity(target_root, &original, &config, action)?;
    write_config(target_root, &config)
}

fn apply_template_mode(
    config: &mut ToolConfig,
    action: &str,
    source_root: &Path,
    target_root: &Path,
) -> Result<()> {
    if action == "create" {
        config.template.is_canonical = false;
        config.template.scaffold_version = Some(latest_scaffold_version());
        return Ok(());
    }

    if action == "init" {
        let canonical_source = source_root.canonicalize()?;
        let canonical_target = target_root.canonicalize()?;
        if canonical_source != canonical_target {
            config.template.is_canonical = false;
        }
    }

    config.template.scaffold_version = Some(latest_scaffold_version());

    Ok(())
}

fn apply_overrides(config: &mut ToolConfig, overrides: &IdentityOverrides) {
    if let Some(value) = &overrides.name {
        config.project.name = value.clone();
    }
    if let Some(value) = &overrides.description {
        config.project.description = value.clone();
    }
    if let Some(value) = &overrides.repository {
        config.project.repository = value.clone();
    }
    if let Some(value) = &overrides.license {
        config.project.license = value.clone();
    }
    if let Some(value) = &overrides.author_name {
        config.author.name = value.clone();
    }
    if let Some(value) = &overrides.author_email {
        config.author.email = value.clone();
    }
    if let Some(value) = &overrides.command {
        config.packages.command = value.clone();
    }
    if let Some(value) = &overrides.rust_crate {
        config.packages.rust_crate = value.clone();
    }
    if let Some(value) = &overrides.npm_package {
        config.packages.npm_package = value.clone();
    }
    if let Some(value) = &overrides.python_package {
        config.packages.python_package = value.clone();
    }
    if overrides.command.is_some() && overrides.rust_crate.is_none() {
        config.packages.rust_crate = config.packages.command.clone();
    }
    if overrides.command.is_some() && overrides.npm_package.is_none() {
        config.packages.npm_package = config.packages.command.clone();
    }
    if overrides.command.is_some() && overrides.python_package.is_none() {
        config.packages.python_package = config.packages.command.clone();
    }
}

fn apply_generated_identity_defaults(
    config: &mut ToolConfig,
    source_config: &ToolConfig,
    overrides: &IdentityOverrides,
) {
    if !source_config.template.is_canonical {
        return;
    }

    if overrides.description.is_none()
        && config.project.description == source_config.project.description
    {
        config.project.description = generated_project_description(&config.packages.command);
    }
    if overrides.repository.is_none()
        && config.project.repository == source_config.project.repository
    {
        config.project.repository = GENERATED_REPOSITORY_PLACEHOLDER.to_string();
    }
    if overrides.author_name.is_none() && config.author.name == source_config.author.name {
        config.author.name = GENERATED_AUTHOR_NAME_PLACEHOLDER.to_string();
    }
    if overrides.author_email.is_none() && config.author.email == source_config.author.email {
        config.author.email = GENERATED_AUTHOR_EMAIL_PLACEHOLDER.to_string();
    }
}

fn relocate_generated_identity_paths(
    target_root: &Path,
    original: &ToolConfig,
    updated: &ToolConfig,
) -> Result<()> {
    relocate_path(
        target_root,
        wrapper_js_launcher_path(original),
        wrapper_js_launcher_path(updated),
    )?;
    relocate_path(
        target_root,
        wrapper_py_package_dir(original),
        wrapper_py_package_dir(updated),
    )?;
    Ok(())
}

fn normalize_cargo_lock_identity(
    target_root: &Path,
    original: &ToolConfig,
    updated: &ToolConfig,
) -> Result<()> {
    if original.packages.rust_crate == updated.packages.rust_crate {
        return Ok(());
    }

    let lock_path = target_root.join("core-rs/Cargo.lock");
    if !lock_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&lock_path)?;
    let old_name = format!("name = \"{}\"", original.packages.rust_crate);
    let new_name = format!("name = \"{}\"", updated.packages.rust_crate);
    if !content.contains(&old_name) {
        return Ok(());
    }

    fs::write(lock_path, content.replacen(&old_name, &new_name, 1))?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct RuntimeTargetsFile {
    targets: Vec<RuntimeTargetSpec>,
}

#[derive(Debug, Clone, Deserialize)]
struct RuntimeTargetSpec {
    #[serde(rename = "packageSuffix")]
    package_suffix: String,
    os: String,
    cpu: String,
}

fn normalize_package_lock_identity(
    target_root: &Path,
    original: &ToolConfig,
    updated: &ToolConfig,
    action: &str,
) -> Result<()> {
    let lock_path = target_root.join("wrapper-js/package-lock.json");
    if !lock_path.exists() {
        return Ok(());
    }

    let reset_runtime_resolution =
        action == "create" || original.packages.npm_package != updated.packages.npm_package;

    let runtime_targets: RuntimeTargetsFile = serde_json::from_str(
        &fs::read_to_string(target_root.join("runtime-targets.json"))
            .context("failed to read runtime-targets.json for package-lock normalization")?,
    )
    .context("failed to parse runtime-targets.json for package-lock normalization")?;

    let mut value: serde_json::Value = serde_json::from_str(&fs::read_to_string(&lock_path)?)
        .context("failed to parse wrapper-js/package-lock.json")?;

    let version = value
        .get("version")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    value["name"] = serde_json::Value::String(updated.packages.npm_package.clone());

    let packages = value
        .get_mut("packages")
        .and_then(serde_json::Value::as_object_mut)
        .context("wrapper-js/package-lock.json is missing packages")?;

    let root_package = packages
        .get_mut("")
        .and_then(serde_json::Value::as_object_mut)
        .context("wrapper-js/package-lock.json is missing packages[\"\"]")?;
    root_package.insert(
        "name".into(),
        serde_json::Value::String(updated.packages.npm_package.clone()),
    );
    root_package.insert(
        "license".into(),
        serde_json::Value::String(updated.project.license.clone()),
    );
    root_package.insert(
        "bin".into(),
        json!({ updated.packages.command.clone(): format!("bin/{}.js", updated.packages.command) }),
    );
    root_package.insert(
        "optionalDependencies".into(),
        serde_json::Value::Object(serde_json::Map::from_iter(
            runtime_targets.targets.iter().map(|spec| {
                (
                    runtime_package_name(&updated.packages.npm_package, spec),
                    serde_json::Value::String(version.clone()),
                )
            }),
        )),
    );

    for spec in &runtime_targets.targets {
        let old_entry_path = format!(
            "node_modules/{}",
            runtime_package_name(&original.packages.npm_package, spec)
        );
        let new_entry_path = format!(
            "node_modules/{}",
            runtime_package_name(&updated.packages.npm_package, spec)
        );
        let mut entry = packages
            .remove(&old_entry_path)
            .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));
        let entry_object = entry
            .as_object_mut()
            .context("runtime package entry in package-lock must be a JSON object")?;
        entry_object.insert("version".into(), serde_json::Value::String(version.clone()));
        entry_object.insert(
            "license".into(),
            serde_json::Value::String(updated.project.license.clone()),
        );
        entry_object.insert("optional".into(), serde_json::Value::Bool(true));
        entry_object.insert("os".into(), json!([spec.os.clone()]));
        entry_object.insert("cpu".into(), json!([spec.cpu.clone()]));
        if reset_runtime_resolution {
            entry_object.remove("resolved");
            entry_object.remove("integrity");
        }
        packages.insert(new_entry_path, entry);
    }

    let mut rendered = serde_json::to_string_pretty(&value)?;
    rendered.push('\n');
    fs::write(lock_path, rendered)?;
    Ok(())
}

fn remove_generated_python_runtime_dirs(
    target_root: &Path,
    original: &ToolConfig,
    updated: &ToolConfig,
) -> Result<()> {
    for package_dir in [
        wrapper_py_package_dir(original),
        wrapper_py_package_dir(updated),
    ] {
        let generated_bin_dir = target_root.join(package_dir).join("bin");
        if generated_bin_dir.exists() {
            fs::remove_dir_all(&generated_bin_dir)?;
        }
    }
    Ok(())
}

fn runtime_package_name(root_package_name: &str, spec: &RuntimeTargetSpec) -> String {
    format!("{}-{}", root_package_name, spec.package_suffix)
}

fn relocate_path(target_root: &Path, original: PathBuf, updated: PathBuf) -> Result<()> {
    if original == updated {
        return Ok(());
    }

    let original_path = target_root.join(&original);
    let updated_path = target_root.join(&updated);
    if !original_path.exists() || updated_path.exists() {
        return Ok(());
    }

    if let Some(parent) = updated_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(original_path, updated_path)?;
    Ok(())
}

fn wrapper_js_launcher_path(config: &ToolConfig) -> PathBuf {
    PathBuf::from("wrapper-js")
        .join("bin")
        .join(format!("{}.js", config.packages.command))
}

fn wrapper_py_package_dir(config: &ToolConfig) -> PathBuf {
    PathBuf::from("wrapper-py")
        .join("src")
        .join(python_module_name(config))
}

fn python_module_name(config: &ToolConfig) -> String {
    config
        .packages
        .python_package
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => ch,
            '-' | '.' => '_',
            _ => '_',
        })
        .collect()
}
