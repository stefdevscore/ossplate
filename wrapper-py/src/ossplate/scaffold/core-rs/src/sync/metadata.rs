use crate::config::ToolConfig;
use crate::sync::{issue, ManagedFile, ValidationIssue};
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use serde_json::json;
use toml::Value as TomlValue;

#[derive(Debug, Clone, Deserialize)]
struct RuntimePackageSpec {
    target: String,
    #[serde(rename = "folderSuffix")]
    folder_suffix: String,
    #[serde(rename = "packageSuffix")]
    package_suffix: String,
    os: String,
    cpu: String,
}

#[derive(Debug, Deserialize)]
struct RuntimeTargetsFile {
    targets: Vec<RuntimePackageSpec>,
}

fn runtime_package_specs() -> Vec<RuntimePackageSpec> {
    serde_json::from_str::<RuntimeTargetsFile>(include_str!("../../runtime-targets.json"))
        .expect("runtime-targets.json must parse")
        .targets
}

fn rust_bin_file_path() -> &'static str {
    "src/main.rs"
}

fn js_wrapper_launcher_path(config: &ToolConfig) -> String {
    format!("bin/{}.js", config.packages.command)
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

fn python_package_src_dir(config: &ToolConfig) -> String {
    format!("src/{}", python_module_name(config))
}

fn python_entrypoint(config: &ToolConfig) -> String {
    format!("{}.cli:main", python_module_name(config))
}

fn toml_string_array(values: &[String]) -> TomlValue {
    TomlValue::Array(
        values
            .iter()
            .cloned()
            .map(TomlValue::String)
            .collect::<Vec<_>>(),
    )
}

fn json_string_array(values: &[String]) -> serde_json::Value {
    serde_json::Value::Array(
        values
            .iter()
            .cloned()
            .map(serde_json::Value::String)
            .collect::<Vec<_>>(),
    )
}

pub(crate) fn validate_cargo_toml(
    config: &ToolConfig,
    content: &str,
) -> Result<Vec<ValidationIssue>> {
    let value: TomlValue = toml::from_str(content).context("failed to parse core-rs/Cargo.toml")?;
    let package = value
        .get("package")
        .and_then(TomlValue::as_table)
        .ok_or_else(|| anyhow!("missing [package] table in core-rs/Cargo.toml"))?;

    let mut issues = Vec::new();
    check_string_field(
        &mut issues,
        "core-rs/Cargo.toml",
        "package.name",
        package.get("name"),
        &config.packages.rust_crate,
    );
    check_string_field(
        &mut issues,
        "core-rs/Cargo.toml",
        "package.default-run",
        package.get("default-run"),
        &config.packages.command,
    );
    check_string_field(
        &mut issues,
        "core-rs/Cargo.toml",
        "package.description",
        package.get("description"),
        &config.project.description,
    );
    check_string_field(
        &mut issues,
        "core-rs/Cargo.toml",
        "package.license",
        package.get("license"),
        &config.project.license,
    );
    check_string_field(
        &mut issues,
        "core-rs/Cargo.toml",
        "package.repository",
        package.get("repository"),
        &config.project.repository,
    );
    check_string_field(
        &mut issues,
        "core-rs/Cargo.toml",
        "package.homepage",
        package.get("homepage"),
        &config.project.repository,
    );
    check_string_array_field(
        &mut issues,
        "core-rs/Cargo.toml",
        "package.keywords",
        package.get("keywords"),
        &config.metadata.rust_keywords,
    );
    check_string_array_field(
        &mut issues,
        "core-rs/Cargo.toml",
        "package.categories",
        package.get("categories"),
        &config.metadata.rust_categories,
    );
    let expected_author = format!("{} <{}>", config.author.name, config.author.email);
    let actual_author = package
        .get("authors")
        .and_then(TomlValue::as_array)
        .and_then(|items| items.first())
        .and_then(TomlValue::as_str)
        .unwrap_or_default()
        .to_string();
    if actual_author != expected_author {
        issues.push(issue(
            "core-rs/Cargo.toml",
            "package.authors",
            "owned metadata differs from the canonical project identity",
            Some(expected_author),
            Some(actual_author),
        ));
    }
    if let Some(bin) = value
        .get("bin")
        .and_then(TomlValue::as_array)
        .and_then(|bins| bins.first())
        .and_then(TomlValue::as_table)
    {
        check_string_field(
            &mut issues,
            "core-rs/Cargo.toml",
            "bin[0].name",
            bin.get("name"),
            &config.packages.command,
        );
        check_string_field(
            &mut issues,
            "core-rs/Cargo.toml",
            "bin[0].path",
            bin.get("path"),
            rust_bin_file_path(),
        );
    } else {
        issues.push(issue(
            "core-rs/Cargo.toml",
            "bin",
            "owned metadata differs from the canonical project identity",
            Some(format!(
                "[[bin]] name={}, path={}",
                config.packages.command,
                rust_bin_file_path()
            )),
            None,
        ));
    }
    Ok(issues)
}

pub(crate) fn sync_cargo_toml(config: &ToolConfig, content: &str) -> Result<String> {
    let mut value: TomlValue =
        toml::from_str(content).context("failed to parse core-rs/Cargo.toml")?;
    let package = value
        .get_mut("package")
        .and_then(TomlValue::as_table_mut)
        .ok_or_else(|| anyhow!("missing [package] table in core-rs/Cargo.toml"))?;
    package.insert(
        "name".into(),
        TomlValue::String(config.packages.rust_crate.clone()),
    );
    package.insert(
        "default-run".into(),
        TomlValue::String(config.packages.command.clone()),
    );
    package.insert(
        "authors".into(),
        TomlValue::Array(vec![TomlValue::String(format!(
            "{} <{}>",
            config.author.name, config.author.email
        ))]),
    );
    package.insert(
        "description".into(),
        TomlValue::String(config.project.description.clone()),
    );
    package.insert(
        "license".into(),
        TomlValue::String(config.project.license.clone()),
    );
    package.insert(
        "repository".into(),
        TomlValue::String(config.project.repository.clone()),
    );
    package.insert(
        "homepage".into(),
        TomlValue::String(config.project.repository.clone()),
    );
    package.insert(
        "keywords".into(),
        toml_string_array(&config.metadata.rust_keywords),
    );
    package.insert(
        "categories".into(),
        toml_string_array(&config.metadata.rust_categories),
    );
    let mut bin = toml::map::Map::new();
    bin.insert(
        "name".into(),
        TomlValue::String(config.packages.command.clone()),
    );
    bin.insert(
        "path".into(),
        TomlValue::String(rust_bin_file_path().to_string()),
    );
    value
        .as_table_mut()
        .ok_or_else(|| anyhow!("core-rs/Cargo.toml must be a TOML table"))?
        .insert("bin".into(), TomlValue::Array(vec![TomlValue::Table(bin)]));
    Ok(toml::to_string(&value)?)
}

pub(crate) fn validate_package_json(
    config: &ToolConfig,
    content: &str,
) -> Result<Vec<ValidationIssue>> {
    let value: serde_json::Value =
        serde_json::from_str(content).context("failed to parse wrapper-js/package.json")?;
    let mut issues = Vec::new();
    check_json_string(
        &mut issues,
        "wrapper-js/package.json",
        "name",
        value.get("name"),
        &config.packages.npm_package,
    );
    check_json_string(
        &mut issues,
        "wrapper-js/package.json",
        "description",
        value.get("description"),
        &config.project.description,
    );
    check_json_string(
        &mut issues,
        "wrapper-js/package.json",
        "author",
        value.get("author"),
        &format!("{} <{}>", config.author.name, config.author.email),
    );
    check_json_string(
        &mut issues,
        "wrapper-js/package.json",
        "license",
        value.get("license"),
        &config.project.license,
    );
    let repo_url = value
        .get("repository")
        .and_then(|v| v.get("url"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    if repo_url != config.project.repository {
        issues.push(issue(
            "wrapper-js/package.json",
            "repository.url",
            "owned metadata differs from the canonical project identity",
            Some(config.project.repository.clone()),
            Some(repo_url),
        ));
    }
    let bin_target = value
        .get("bin")
        .and_then(|v| v.get(&config.packages.command))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    let expected_bin_target = js_wrapper_launcher_path(config);
    if bin_target != expected_bin_target {
        issues.push(issue(
            "wrapper-js/package.json",
            "bin",
            "owned metadata differs from the canonical project identity",
            Some(expected_bin_target),
            Some(bin_target),
        ));
    }
    let package_version = value
        .get("version")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let expected_optional_dependencies =
        serde_json::Map::from_iter(runtime_package_specs().into_iter().map(|spec| {
            (
                runtime_package_name(config, &spec.target),
                serde_json::Value::String(package_version.to_string()),
            )
        }));
    let actual_optional_dependencies = value
        .get("optionalDependencies")
        .and_then(serde_json::Value::as_object)
        .cloned()
        .unwrap_or_default();
    if actual_optional_dependencies != expected_optional_dependencies {
        issues.push(issue(
            "wrapper-js/package.json",
            "optionalDependencies",
            "owned metadata differs from the canonical project identity",
            Some(serde_json::to_string_pretty(
                &expected_optional_dependencies,
            )?),
            Some(serde_json::to_string_pretty(&actual_optional_dependencies)?),
        ));
    }
    let actual_keywords = value
        .get("keywords")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let expected_keywords = config
        .metadata
        .npm_keywords
        .iter()
        .cloned()
        .map(serde_json::Value::String)
        .collect::<Vec<_>>();
    if actual_keywords != expected_keywords {
        issues.push(issue(
            "wrapper-js/package.json",
            "keywords",
            "owned metadata differs from the canonical project identity",
            Some(serde_json::to_string_pretty(&expected_keywords)?),
            Some(serde_json::to_string_pretty(&actual_keywords)?),
        ));
    }
    Ok(issues)
}

pub(crate) fn sync_package_json(config: &ToolConfig, content: &str) -> Result<String> {
    let mut value: serde_json::Value =
        serde_json::from_str(content).context("failed to parse wrapper-js/package.json")?;
    value["name"] = serde_json::Value::String(config.packages.npm_package.clone());
    value["description"] = serde_json::Value::String(config.project.description.clone());
    value["author"] =
        serde_json::Value::String(format!("{} <{}>", config.author.name, config.author.email));
    value["license"] = serde_json::Value::String(config.project.license.clone());
    value["repository"]["url"] = serde_json::Value::String(config.project.repository.clone());
    value["bin"] = json!({
        config.packages.command.clone(): js_wrapper_launcher_path(config)
    });
    value["keywords"] = json_string_array(&config.metadata.npm_keywords);
    let version = value
        .get("version")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    value["optionalDependencies"] = serde_json::Value::Object(serde_json::Map::from_iter(
        runtime_package_specs().into_iter().map(|spec| {
            (
                runtime_package_name(config, &spec.target),
                serde_json::Value::String(version.clone()),
            )
        }),
    ));
    let mut rendered = serde_json::to_string_pretty(&value)?;
    rendered.push('\n');
    Ok(rendered)
}

fn runtime_package_spec(target: &str) -> RuntimePackageSpec {
    runtime_package_specs()
        .into_iter()
        .find(|spec| spec.target == target)
        .unwrap_or_else(|| panic!("unsupported runtime package target: {target}"))
}

fn runtime_package_manifest_path(target: &str) -> &'static str {
    match target {
        "darwin-arm64" => "wrapper-js/platform-packages/ossplate-darwin-arm64/package.json",
        "darwin-x64" => "wrapper-js/platform-packages/ossplate-darwin-x64/package.json",
        "linux-x64" => "wrapper-js/platform-packages/ossplate-linux-x64/package.json",
        "win32-x64" => "wrapper-js/platform-packages/ossplate-win32-x64/package.json",
        other => panic!("unsupported runtime package target: {other}"),
    }
}

fn runtime_package_folder(config: &ToolConfig, target: &str) -> String {
    let spec = runtime_package_spec(target);
    format!("{}-{}", config.packages.npm_package, spec.folder_suffix)
}

fn runtime_package_name(config: &ToolConfig, target: &str) -> String {
    let spec = runtime_package_spec(target);
    format!("{}-{}", config.packages.npm_package, spec.package_suffix)
}

fn runtime_binary_name(config: &ToolConfig, target: &str) -> String {
    let spec = runtime_package_spec(target);
    if spec.os == "win32" {
        format!("{}.exe", config.packages.command)
    } else {
        config.packages.command.clone()
    }
}

fn validate_runtime_package_json(
    config: &ToolConfig,
    content: &str,
    spec: RuntimePackageSpec,
) -> Result<Vec<ValidationIssue>> {
    let value: serde_json::Value = serde_json::from_str(content).with_context(|| {
        format!(
            "failed to parse {}",
            runtime_package_manifest_path(&spec.target)
        )
    })?;
    let mut issues = Vec::new();
    check_json_string(
        &mut issues,
        runtime_package_manifest_path(&spec.target),
        "name",
        value.get("name"),
        &runtime_package_name(config, &spec.target),
    );
    check_json_string(
        &mut issues,
        runtime_package_manifest_path(&spec.target),
        "description",
        value.get("description"),
        &format!(
            "Platform runtime package for {} on {}.",
            config.packages.npm_package, spec.target
        ),
    );
    check_json_string(
        &mut issues,
        runtime_package_manifest_path(&spec.target),
        "license",
        value.get("license"),
        &config.project.license,
    );
    check_json_string(
        &mut issues,
        runtime_package_manifest_path(&spec.target),
        "repository.url",
        value.get("repository").and_then(|v| v.get("url")),
        &config.project.repository,
    );
    check_json_string(
        &mut issues,
        runtime_package_manifest_path(&spec.target),
        "repository.directory",
        value.get("repository").and_then(|v| v.get("directory")),
        &format!(
            "wrapper-js/platform-packages/{}",
            runtime_package_folder(config, &spec.target)
        ),
    );
    check_json_string(
        &mut issues,
        runtime_package_manifest_path(&spec.target),
        "os[0]",
        value.get("os").and_then(|v| v.get(0)),
        &spec.os,
    );
    check_json_string(
        &mut issues,
        runtime_package_manifest_path(&spec.target),
        "cpu[0]",
        value.get("cpu").and_then(|v| v.get(0)),
        &spec.cpu,
    );
    Ok(issues)
}

fn sync_runtime_package_json(
    config: &ToolConfig,
    content: &str,
    spec: RuntimePackageSpec,
) -> Result<String> {
    let mut value: serde_json::Value = serde_json::from_str(content).with_context(|| {
        format!(
            "failed to parse {}",
            runtime_package_manifest_path(&spec.target)
        )
    })?;
    let package_name = runtime_package_name(config, &spec.target);
    let package_folder = runtime_package_folder(config, &spec.target);
    value["name"] = serde_json::Value::String(package_name.clone());
    value["description"] = serde_json::Value::String(format!(
        "Platform runtime package for {} on {}.",
        config.packages.npm_package, spec.target
    ));
    value["license"] = serde_json::Value::String(config.project.license.clone());
    value["repository"]["url"] = serde_json::Value::String(config.project.repository.clone());
    value["repository"]["directory"] =
        serde_json::Value::String(format!("wrapper-js/platform-packages/{package_folder}"));
    value["os"] = json!([spec.os]);
    value["cpu"] = json!([spec.cpu]);
    let mut rendered = serde_json::to_string_pretty(&value)?;
    rendered.push('\n');
    Ok(rendered)
}

macro_rules! runtime_package_handlers {
    ($(($validate:ident, $sync:ident, $target:literal)),+ $(,)?) => {
        $(
            pub(crate) fn $validate(config: &ToolConfig, content: &str) -> Result<Vec<ValidationIssue>> {
                validate_runtime_package_json(config, content, runtime_package_spec($target))
            }

            pub(crate) fn $sync(config: &ToolConfig, content: &str) -> Result<String> {
                sync_runtime_package_json(config, content, runtime_package_spec($target))
            }
        )+
    };
}

runtime_package_handlers!(
    (
        validate_runtime_package_json_darwin_arm64,
        sync_runtime_package_json_darwin_arm64,
        "darwin-arm64"
    ),
    (
        validate_runtime_package_json_darwin_x64,
        sync_runtime_package_json_darwin_x64,
        "darwin-x64"
    ),
    (
        validate_runtime_package_json_linux_x64,
        sync_runtime_package_json_linux_x64,
        "linux-x64"
    ),
    (
        validate_runtime_package_json_win32_x64,
        sync_runtime_package_json_win32_x64,
        "win32-x64"
    ),
);

pub(crate) fn runtime_package_managed_files() -> Vec<ManagedFile> {
    vec![
        ManagedFile {
            path: runtime_package_manifest_path("darwin-arm64"),
            validate: validate_runtime_package_json_darwin_arm64,
            sync: sync_runtime_package_json_darwin_arm64,
        },
        ManagedFile {
            path: runtime_package_manifest_path("darwin-x64"),
            validate: validate_runtime_package_json_darwin_x64,
            sync: sync_runtime_package_json_darwin_x64,
        },
        ManagedFile {
            path: runtime_package_manifest_path("linux-x64"),
            validate: validate_runtime_package_json_linux_x64,
            sync: sync_runtime_package_json_linux_x64,
        },
        ManagedFile {
            path: runtime_package_manifest_path("win32-x64"),
            validate: validate_runtime_package_json_win32_x64,
            sync: sync_runtime_package_json_win32_x64,
        },
    ]
}

fn validate_runtime_targets_json_at_path(
    config: &ToolConfig,
    content: &str,
    file_path: &str,
) -> Result<Vec<ValidationIssue>> {
    let value: serde_json::Value =
        serde_json::from_str(content).with_context(|| format!("failed to parse {file_path}"))?;
    let targets = value
        .get("targets")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow!("missing targets in {file_path}"))?;
    let mut issues = Vec::new();

    for target in targets {
        let target_name = target
            .get("target")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow!("runtime target is missing target name"))?;
        let actual_binary = target
            .get("binary")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        let expected_binary = runtime_binary_name(config, target_name);
        if actual_binary != expected_binary {
            issues.push(issue(
                file_path,
                &format!("targets.{target_name}.binary"),
                "owned metadata differs from the canonical project identity",
                Some(expected_binary),
                Some(actual_binary),
            ));
        }
    }

    Ok(issues)
}

pub(crate) fn validate_runtime_targets_json(
    config: &ToolConfig,
    content: &str,
) -> Result<Vec<ValidationIssue>> {
    validate_runtime_targets_json_at_path(config, content, "runtime-targets.json")
}

pub(crate) fn validate_core_runtime_targets_json(
    config: &ToolConfig,
    content: &str,
) -> Result<Vec<ValidationIssue>> {
    validate_runtime_targets_json_at_path(config, content, "core-rs/runtime-targets.json")
}

pub(crate) fn sync_runtime_targets_json(config: &ToolConfig, content: &str) -> Result<String> {
    let mut value: serde_json::Value =
        serde_json::from_str(content).context("failed to parse runtime-targets.json")?;
    let targets = value
        .get_mut("targets")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or_else(|| anyhow!("missing targets in runtime-targets.json"))?;

    for target in targets {
        let target_name = target
            .get("target")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow!("runtime target is missing target name"))?
            .to_string();
        target["binary"] = serde_json::Value::String(runtime_binary_name(config, &target_name));
    }

    let mut rendered = serde_json::to_string_pretty(&value)?;
    rendered.push('\n');
    Ok(rendered)
}

pub(crate) fn validate_scaffold_payload_json(
    config: &ToolConfig,
    content: &str,
) -> Result<Vec<ValidationIssue>> {
    let value: serde_json::Value =
        serde_json::from_str(content).context("failed to parse scaffold-payload.json")?;
    let normalized = normalize_scaffold_payload_json(config, value.clone())?;
    let mut issues = Vec::new();
    if value != normalized {
        issues.push(issue(
            "scaffold-payload.json",
            "requiredPaths",
            "owned metadata differs from the canonical project identity",
            Some(serde_json::to_string_pretty(&normalized)?),
            Some(serde_json::to_string_pretty(&value)?),
        ));
    }
    Ok(issues)
}

pub(crate) fn sync_scaffold_payload_json(config: &ToolConfig, content: &str) -> Result<String> {
    let value: serde_json::Value =
        serde_json::from_str(content).context("failed to parse scaffold-payload.json")?;
    let normalized = normalize_scaffold_payload_json(config, value)?;
    let mut rendered = serde_json::to_string_pretty(&normalized)?;
    rendered.push('\n');
    Ok(rendered)
}

pub(crate) fn validate_pyproject(
    config: &ToolConfig,
    content: &str,
) -> Result<Vec<ValidationIssue>> {
    let value: TomlValue =
        toml::from_str(content).context("failed to parse wrapper-py/pyproject.toml")?;
    let project = value
        .get("project")
        .and_then(TomlValue::as_table)
        .ok_or_else(|| anyhow!("missing [project] table in wrapper-py/pyproject.toml"))?;
    let mut issues = Vec::new();
    check_string_field(
        &mut issues,
        "wrapper-py/pyproject.toml",
        "project.name",
        project.get("name"),
        &config.packages.python_package,
    );
    check_string_field(
        &mut issues,
        "wrapper-py/pyproject.toml",
        "project.description",
        project.get("description"),
        &config.project.description,
    );
    let license = project
        .get("license")
        .and_then(TomlValue::as_table)
        .and_then(|t| t.get("text"));
    check_string_field(
        &mut issues,
        "wrapper-py/pyproject.toml",
        "project.license.text",
        license,
        &config.project.license,
    );
    let author = project
        .get("authors")
        .and_then(TomlValue::as_array)
        .and_then(|items| items.first())
        .and_then(TomlValue::as_table);
    let actual_name = author
        .and_then(|item| item.get("name"))
        .and_then(TomlValue::as_str)
        .unwrap_or_default()
        .to_string();
    if actual_name != config.author.name {
        issues.push(issue(
            "wrapper-py/pyproject.toml",
            "project.authors[0].name",
            "owned metadata differs from the canonical project identity",
            Some(config.author.name.clone()),
            Some(actual_name),
        ));
    }
    let actual_email = author
        .and_then(|item| item.get("email"))
        .and_then(TomlValue::as_str)
        .unwrap_or_default()
        .to_string();
    if actual_email != config.author.email {
        issues.push(issue(
            "wrapper-py/pyproject.toml",
            "project.authors[0].email",
            "owned metadata differs from the canonical project identity",
            Some(config.author.email.clone()),
            Some(actual_email),
        ));
    }
    check_string_array_field(
        &mut issues,
        "wrapper-py/pyproject.toml",
        "project.keywords",
        project.get("keywords"),
        &config.metadata.python_keywords,
    );
    check_string_array_field(
        &mut issues,
        "wrapper-py/pyproject.toml",
        "project.classifiers",
        project.get("classifiers"),
        &config.metadata.python_classifiers,
    );
    let urls = project_urls(&value)?;
    if urls.0 != config.project.repository {
        issues.push(issue(
            "wrapper-py/pyproject.toml",
            "project.urls.Homepage",
            "owned metadata differs from the canonical project identity",
            Some(config.project.repository.clone()),
            Some(urls.0),
        ));
    }
    if urls.1 != config.project.repository {
        issues.push(issue(
            "wrapper-py/pyproject.toml",
            "project.urls.Repository",
            "owned metadata differs from the canonical project identity",
            Some(config.project.repository.clone()),
            Some(urls.1),
        ));
    }
    let scripts = value
        .get("project")
        .and_then(TomlValue::as_table)
        .and_then(|t| t.get("scripts"))
        .and_then(TomlValue::as_table)
        .ok_or_else(|| anyhow!("missing [project.scripts] in wrapper-py/pyproject.toml"))?;
    let actual_entry = scripts
        .get(&config.packages.command)
        .and_then(TomlValue::as_str)
        .unwrap_or_default()
        .to_string();
    let expected_entry = python_entrypoint(config);
    if actual_entry != expected_entry {
        issues.push(issue(
            "wrapper-py/pyproject.toml",
            "project.scripts",
            "owned metadata differs from the canonical project identity",
            Some(expected_entry),
            Some(actual_entry),
        ));
    }
    let wheel = value
        .as_table()
        .and_then(|t| t.get("tool"))
        .and_then(TomlValue::as_table)
        .and_then(|t| t.get("hatch"))
        .and_then(TomlValue::as_table)
        .and_then(|t| t.get("build"))
        .and_then(TomlValue::as_table)
        .and_then(|t| t.get("targets"))
        .and_then(TomlValue::as_table)
        .and_then(|t| t.get("wheel"))
        .and_then(TomlValue::as_table)
        .ok_or_else(|| {
            anyhow!("missing [tool.hatch.build.targets.wheel] in wrapper-py/pyproject.toml")
        })?;
    let expected_package_dir = python_package_src_dir(config);
    let actual_packages = wheel
        .get("packages")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    let expected_packages = vec![TomlValue::String(expected_package_dir.clone())];
    if actual_packages != expected_packages {
        issues.push(issue(
            "wrapper-py/pyproject.toml",
            "tool.hatch.build.targets.wheel.packages",
            "owned metadata differs from the canonical project identity",
            Some(render_toml_string_array(&expected_packages)),
            Some(render_toml_string_array(&actual_packages)),
        ));
    }
    let actual_artifacts = wheel
        .get("artifacts")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    let expected_artifacts = vec![TomlValue::String(format!(
        "{expected_package_dir}/scaffold/**"
    ))];
    if actual_artifacts != expected_artifacts {
        issues.push(issue(
            "wrapper-py/pyproject.toml",
            "tool.hatch.build.targets.wheel.artifacts",
            "owned metadata differs from the canonical project identity",
            Some(render_toml_string_array(&expected_artifacts)),
            Some(render_toml_string_array(&actual_artifacts)),
        ));
    }
    let actual_exclude = wheel
        .get("exclude")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    let expected_exclude = vec![TomlValue::String(format!("{expected_package_dir}/bin/**"))];
    if actual_exclude != expected_exclude {
        issues.push(issue(
            "wrapper-py/pyproject.toml",
            "tool.hatch.build.targets.wheel.exclude",
            "owned metadata differs from the canonical project identity",
            Some(render_toml_string_array(&expected_exclude)),
            Some(render_toml_string_array(&actual_exclude)),
        ));
    }
    Ok(issues)
}

pub(crate) fn sync_pyproject(config: &ToolConfig, content: &str) -> Result<String> {
    let mut value: TomlValue =
        toml::from_str(content).context("failed to parse wrapper-py/pyproject.toml")?;
    let project = value
        .get_mut("project")
        .and_then(TomlValue::as_table_mut)
        .ok_or_else(|| anyhow!("missing [project] table in wrapper-py/pyproject.toml"))?;
    project.insert(
        "name".into(),
        TomlValue::String(config.packages.python_package.clone()),
    );
    project.insert(
        "description".into(),
        TomlValue::String(config.project.description.clone()),
    );
    let mut license = toml::map::Map::new();
    license.insert(
        "text".into(),
        TomlValue::String(config.project.license.clone()),
    );
    project.insert("license".into(), TomlValue::Table(license));
    let mut author = toml::map::Map::new();
    author.insert("name".into(), TomlValue::String(config.author.name.clone()));
    author.insert(
        "email".into(),
        TomlValue::String(config.author.email.clone()),
    );
    project.insert(
        "authors".into(),
        TomlValue::Array(vec![TomlValue::Table(author)]),
    );
    project.insert(
        "keywords".into(),
        toml_string_array(&config.metadata.python_keywords),
    );
    project.insert(
        "classifiers".into(),
        toml_string_array(&config.metadata.python_classifiers),
    );
    let mut scripts = toml::map::Map::new();
    scripts.insert(
        config.packages.command.clone(),
        TomlValue::String(python_entrypoint(config)),
    );
    project.insert("scripts".into(), TomlValue::Table(scripts));
    let urls = value
        .as_table_mut()
        .and_then(|t| t.get_mut("project"))
        .and_then(TomlValue::as_table_mut)
        .and_then(|t| t.get_mut("urls"))
        .and_then(TomlValue::as_table_mut)
        .ok_or_else(|| anyhow!("missing [project.urls] in wrapper-py/pyproject.toml"))?;
    urls.insert(
        "Homepage".into(),
        TomlValue::String(config.project.repository.clone()),
    );
    urls.insert(
        "Repository".into(),
        TomlValue::String(config.project.repository.clone()),
    );
    let wheel = value
        .as_table_mut()
        .and_then(|t| t.get_mut("tool"))
        .and_then(TomlValue::as_table_mut)
        .and_then(|t| t.get_mut("hatch"))
        .and_then(TomlValue::as_table_mut)
        .and_then(|t| t.get_mut("build"))
        .and_then(TomlValue::as_table_mut)
        .and_then(|t| t.get_mut("targets"))
        .and_then(TomlValue::as_table_mut)
        .and_then(|t| t.get_mut("wheel"))
        .and_then(TomlValue::as_table_mut)
        .ok_or_else(|| {
            anyhow!("missing [tool.hatch.build.targets.wheel] in wrapper-py/pyproject.toml")
        })?;
    let package_dir = python_package_src_dir(config);
    wheel.insert(
        "packages".into(),
        TomlValue::Array(vec![TomlValue::String(package_dir.clone())]),
    );
    wheel.insert(
        "artifacts".into(),
        TomlValue::Array(vec![TomlValue::String(format!(
            "{package_dir}/scaffold/**"
        ))]),
    );
    wheel.insert(
        "exclude".into(),
        TomlValue::Array(vec![TomlValue::String(format!("{package_dir}/bin/**"))]),
    );
    Ok(toml::to_string(&value)?)
}

fn check_string_field(
    issues: &mut Vec<ValidationIssue>,
    file: &str,
    code: &str,
    value: Option<&TomlValue>,
    expected: &str,
) {
    let actual = value
        .and_then(TomlValue::as_str)
        .unwrap_or_default()
        .to_string();
    if actual != expected {
        issues.push(issue(
            file,
            code,
            "owned metadata differs from the canonical project identity",
            Some(expected.to_string()),
            Some(actual),
        ));
    }
}

fn check_string_array_field(
    issues: &mut Vec<ValidationIssue>,
    file: &str,
    code: &str,
    value: Option<&TomlValue>,
    expected: &[String],
) {
    let actual = value
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    let expected_values = expected
        .iter()
        .cloned()
        .map(TomlValue::String)
        .collect::<Vec<_>>();
    if actual != expected_values {
        issues.push(issue(
            file,
            code,
            "owned metadata differs from the canonical project identity",
            Some(render_toml_string_array(&expected_values)),
            Some(render_toml_string_array(&actual)),
        ));
    }
}

fn check_json_string(
    issues: &mut Vec<ValidationIssue>,
    file: &str,
    code: &str,
    value: Option<&serde_json::Value>,
    expected: &str,
) {
    let actual = value
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    if actual != expected {
        issues.push(issue(
            file,
            code,
            "owned metadata differs from the canonical project identity",
            Some(expected.to_string()),
            Some(actual),
        ));
    }
}

fn render_toml_string_array(values: &[TomlValue]) -> String {
    let rendered = values
        .iter()
        .map(|value| format!("\"{}\"", value.as_str().unwrap_or_default()))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{rendered}]")
}

fn project_urls(value: &TomlValue) -> Result<(String, String)> {
    let urls = value
        .as_table()
        .and_then(|t| t.get("project"))
        .and_then(TomlValue::as_table)
        .and_then(|t| t.get("urls"))
        .and_then(TomlValue::as_table)
        .ok_or_else(|| anyhow!("missing [project.urls] in wrapper-py/pyproject.toml"))?;
    Ok((
        urls.get("Homepage")
            .and_then(TomlValue::as_str)
            .unwrap_or_default()
            .to_string(),
        urls.get("Repository")
            .and_then(TomlValue::as_str)
            .unwrap_or_default()
            .to_string(),
    ))
}

fn normalize_scaffold_payload_json(
    config: &ToolConfig,
    mut value: serde_json::Value,
) -> Result<serde_json::Value> {
    let required_paths = value
        .get_mut("requiredPaths")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or_else(|| anyhow!("missing requiredPaths in scaffold-payload.json"))?;

    let expected_js_launcher = format!("wrapper-js/{}", js_wrapper_launcher_path(config));
    let expected_python_dir = python_package_src_dir(config);
    let expected_python_init = format!("wrapper-py/{expected_python_dir}/__init__.py");
    let expected_python_cli = format!("wrapper-py/{expected_python_dir}/cli.py");

    replace_required_path(
        required_paths,
        |path| path.starts_with("wrapper-js/bin/") && path.ends_with(".js"),
        &expected_js_launcher,
    );
    replace_required_path(
        required_paths,
        |path| path.starts_with("wrapper-py/src/") && path.ends_with("/__init__.py"),
        &expected_python_init,
    );
    replace_required_path(
        required_paths,
        |path| path.starts_with("wrapper-py/src/") && path.ends_with("/cli.py"),
        &expected_python_cli,
    );

    Ok(value)
}

fn replace_required_path(
    required_paths: &mut [serde_json::Value],
    matches: impl Fn(&str) -> bool,
    expected: &str,
) {
    if let Some(entry) = required_paths
        .iter_mut()
        .find(|value| value.as_str().is_some_and(&matches))
    {
        *entry = serde_json::Value::String(expected.to_string());
    }
}
