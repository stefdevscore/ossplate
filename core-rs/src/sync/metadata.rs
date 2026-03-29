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
    serde_json::from_str::<RuntimeTargetsFile>(include_str!("../../../runtime-targets.json"))
        .expect("runtime-targets.json must parse")
        .targets
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
    if bin_target != "bin/ossplate.js" {
        issues.push(issue(
            "wrapper-js/package.json",
            "bin",
            "owned metadata differs from the canonical project identity",
            Some("bin/ossplate.js".to_string()),
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
        config.packages.command.clone(): "bin/ossplate.js"
    });
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
    if actual_entry != "ossplate.cli:main" {
        issues.push(issue(
            "wrapper-py/pyproject.toml",
            "project.scripts",
            "owned metadata differs from the canonical project identity",
            Some("ossplate.cli:main".to_string()),
            Some(actual_entry),
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
    let mut scripts = toml::map::Map::new();
    scripts.insert(
        config.packages.command.clone(),
        TomlValue::String("ossplate.cli:main".to_string()),
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
