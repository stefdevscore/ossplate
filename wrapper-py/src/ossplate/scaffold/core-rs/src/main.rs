use anyhow::{anyhow, bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value as TomlValue;

const README_IDENTITY_START: &str = "<!-- ossplate:readme-identity:start -->";
const README_IDENTITY_END: &str = "<!-- ossplate:readme-identity:end -->";
const WORKFLOW_NAME_START: &str = "# ossplate:workflow-name:start";
const WORKFLOW_NAME_END: &str = "# ossplate:workflow-name:end";

#[derive(Parser)]
#[command(name = "ossplate")]
#[command(
    author,
    version,
    about = "Validate and sync a multi-registry OSS scaffold"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Print tool version information
    Version,
    /// Scaffold a new target directory from the current template
    Create {
        target: PathBuf,
        #[command(flatten)]
        overrides: IdentityOverrides,
    },
    /// Initialize or hydrate an existing directory in place
    Init {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[command(flatten)]
        overrides: IdentityOverrides,
    },
    /// Validate project identity and metadata consistency
    Validate {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Synchronize owned metadata surfaces
    Sync {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        check: bool,
    },
}

#[derive(Debug, Clone, Default, Args)]
struct IdentityOverrides {
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    description: Option<String>,
    #[arg(long)]
    repository: Option<String>,
    #[arg(long)]
    license: Option<String>,
    #[arg(long = "author-name")]
    author_name: Option<String>,
    #[arg(long = "author-email")]
    author_email: Option<String>,
    #[arg(long = "rust-crate")]
    rust_crate: Option<String>,
    #[arg(long = "npm-package")]
    npm_package: Option<String>,
    #[arg(long = "python-package")]
    python_package: Option<String>,
    #[arg(long)]
    command: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ToolConfig {
    project: ProjectConfig,
    author: AuthorConfig,
    packages: PackageConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ProjectConfig {
    name: String,
    description: String,
    repository: String,
    license: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AuthorConfig {
    name: String,
    email: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PackageConfig {
    rust_crate: String,
    npm_package: String,
    python_package: String,
    command: String,
}

#[derive(Debug, Clone, Serialize)]
struct VersionOutput {
    tool: &'static str,
    version: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct ValidationOutput {
    ok: bool,
    issues: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct ValidationIssue {
    code: String,
    file: String,
    message: String,
    expected: Option<String>,
    actual: Option<String>,
}

#[derive(Debug, Clone)]
struct SyncChange {
    path: &'static str,
    issues: Vec<ValidationIssue>,
    synced: String,
}

#[derive(Debug, Clone)]
struct ManagedFile {
    path: &'static str,
    validate: fn(&ToolConfig, &str) -> Result<Vec<ValidationIssue>>,
    sync: fn(&ToolConfig, &str) -> Result<String>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("ossplate: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Version => {
            println!(
                "{}",
                serde_json::to_string(&VersionOutput {
                    tool: env!("CARGO_BIN_NAME"),
                    version: env!("CARGO_PKG_VERSION"),
                })?
            );
            Ok(())
        }
        Commands::Create { target, overrides } => create_scaffold(&target, &overrides),
        Commands::Init { path, overrides } => init_scaffold(&path, &overrides),
        Commands::Validate { path, json } => {
            let output = validate_repo(&path)?;
            if json {
                println!("{}", serde_json::to_string(&output)?);
            } else if output.ok {
                println!("validation ok");
            } else {
                println!(
                    "{}",
                    format_human_issues("validation failed:", &output.issues)
                );
            }

            if output.ok {
                Ok(())
            } else {
                bail!("validation failed")
            }
        }
        Commands::Sync { path, check } => sync_repo(&path, check),
    }
}

fn create_scaffold(target: &Path, overrides: &IdentityOverrides) -> Result<()> {
    let source_root = discover_template_root()?;
    ensure_scaffold_source_root(&source_root)?;
    create_scaffold_from(&source_root, target, overrides)
}

fn init_scaffold(target: &Path, overrides: &IdentityOverrides) -> Result<()> {
    let source_root = discover_template_root()?;
    ensure_scaffold_source_root(&source_root)?;
    init_scaffold_from(&source_root, target, overrides)
}

fn create_scaffold_from(
    source_root: &Path,
    target: &Path,
    overrides: &IdentityOverrides,
) -> Result<()> {
    if target.exists() {
        if target.read_dir()?.next().is_some() {
            bail!("target directory is not empty: {}", target.display());
        }
    } else {
        fs::create_dir_all(target)
            .with_context(|| format!("failed to create {}", target.display()))?;
    }

    let source_root = source_root.canonicalize().with_context(|| {
        format!(
            "failed to canonicalize source root {}",
            source_root.display()
        )
    })?;
    let target_root = target
        .canonicalize()
        .with_context(|| format!("failed to canonicalize target root {}", target.display()))?;
    if target_root.starts_with(&source_root) {
        bail!("target directory must not be inside the source template tree");
    }

    copy_tree(&source_root, &target_root)?;
    apply_config_overrides_to_target(&target_root, &source_root, overrides)?;
    sync_repo(&target_root, false)?;
    println!("created scaffold at {}", target_root.display());
    Ok(())
}

fn init_scaffold_from(
    source_root: &Path,
    target: &Path,
    overrides: &IdentityOverrides,
) -> Result<()> {
    if !target.exists() {
        fs::create_dir_all(target)
            .with_context(|| format!("failed to create {}", target.display()))?;
    }

    let source_root = source_root.canonicalize().with_context(|| {
        format!(
            "failed to canonicalize source root {}",
            source_root.display()
        )
    })?;
    let target_root = target
        .canonicalize()
        .with_context(|| format!("failed to canonicalize target root {}", target.display()))?;
    if target_root.starts_with(&source_root) {
        bail!("target directory must not be inside the source template tree");
    }

    ensure_scaffold_layout(&source_root, &target_root)?;
    apply_config_overrides_to_target(&target_root, &source_root, overrides)?;
    sync_repo(&target_root, false)?;
    println!("initialized scaffold at {}", target_root.display());
    Ok(())
}

fn validate_repo(root: &Path) -> Result<ValidationOutput> {
    let config = load_config(root)?;
    let current = collect_current_files(root)?;
    let mut issues = Vec::new();

    for file in &managed_files() {
        let actual = current
            .get(file.path)
            .ok_or_else(|| anyhow!("missing owned file {}", file.path))?;
        issues.extend((file.validate)(&config, actual)?);
    }

    let output = ValidationOutput {
        ok: issues.is_empty(),
        issues,
    };
    Ok(output)
}

fn sync_repo(root: &Path, check: bool) -> Result<()> {
    let drifted = build_sync_changes(root)?;

    if check {
        if drifted.is_empty() {
            println!("sync check ok");
            return Ok(());
        }
        let issues = drifted
            .iter()
            .flat_map(|change| change.issues.iter().cloned())
            .collect::<Vec<_>>();
        println!("{}", format_human_issues("sync check failed:", &issues));
        bail!("sync check failed")
    }

    for change in drifted {
        let target = root.join(change.path);
        fs::write(&target, change.synced)
            .with_context(|| format!("failed to write {}", target.display()))?;
    }

    println!("sync complete");
    Ok(())
}

fn load_config(root: &Path) -> Result<ToolConfig> {
    let contents =
        fs::read_to_string(root.join("ossplate.toml")).context("failed to read ossplate.toml")?;
    toml::from_str(&contents).context("failed to parse ossplate.toml")
}

fn write_config(root: &Path, config: &ToolConfig) -> Result<()> {
    let mut rendered = toml::to_string(config).context("failed to serialize ossplate.toml")?;
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    fs::write(root.join("ossplate.toml"), rendered).context("failed to write ossplate.toml")?;
    Ok(())
}

fn apply_config_overrides_to_target(
    target_root: &Path,
    source_root: &Path,
    overrides: &IdentityOverrides,
) -> Result<()> {
    let mut config = if target_root.join("ossplate.toml").is_file() {
        load_config(target_root)?
    } else {
        load_config(source_root)?
    };

    apply_overrides(&mut config, overrides);
    write_config(target_root, &config)
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
    if let Some(value) = &overrides.rust_crate {
        config.packages.rust_crate = value.clone();
    }
    if let Some(value) = &overrides.npm_package {
        config.packages.npm_package = value.clone();
    }
    if let Some(value) = &overrides.python_package {
        config.packages.python_package = value.clone();
    }
    if let Some(value) = &overrides.command {
        config.packages.command = value.clone();
    }
}

fn build_sync_changes(root: &Path) -> Result<Vec<SyncChange>> {
    let config = load_config(root)?;
    let current = collect_current_files(root)?;
    let mut drifted = Vec::new();

    for file in &managed_files() {
        let actual = current
            .get(file.path)
            .ok_or_else(|| anyhow!("missing owned file {}", file.path))?;
        let issues = (file.validate)(&config, actual)?;
        if !issues.is_empty() {
            drifted.push(SyncChange {
                path: file.path,
                synced: (file.sync)(&config, actual)?,
                issues,
            });
        }
    }

    Ok(drifted)
}

fn collect_current_files(root: &Path) -> Result<BTreeMap<&'static str, String>> {
    let mut files = BTreeMap::new();
    for path in owned_paths() {
        files.insert(
            path,
            fs::read_to_string(root.join(path))
                .with_context(|| format!("failed to read {}", root.join(path).display()))?,
        );
    }
    Ok(files)
}

fn discover_template_root() -> Result<PathBuf> {
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
        .ok_or_else(|| anyhow!("failed to locate template root containing ossplate.toml"))
}

fn ensure_scaffold_source_root(root: &Path) -> Result<()> {
    let required = [
        "ossplate.toml",
        "README.md",
        "core-rs/Cargo.toml",
        "wrapper-js/package.json",
        "wrapper-py/pyproject.toml",
    ];

    let missing: Vec<_> = required
        .iter()
        .filter(|path| !root.join(path).exists())
        .copied()
        .collect();

    if missing.is_empty() {
        return Ok(());
    }

    bail!(
        "create/init require a full scaffold source checkout; missing required scaffold paths: {}",
        missing.join(", ")
    )
}

fn copy_tree(source_root: &Path, target_root: &Path) -> Result<()> {
    for entry in fs::read_dir(source_root)
        .with_context(|| format!("failed to read {}", source_root.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let file_name = entry.file_name();
        if should_skip_copy(&file_name) {
            continue;
        }

        let target_path = target_root.join(&file_name);
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            fs::create_dir_all(&target_path)
                .with_context(|| format!("failed to create {}", target_path.display()))?;
            copy_tree(&source_path, &target_path)?;
        } else if file_type.is_file() {
            fs::copy(&source_path, &target_path).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    source_path.display(),
                    target_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn ensure_scaffold_layout(source_root: &Path, target_root: &Path) -> Result<()> {
    for entry in fs::read_dir(source_root)
        .with_context(|| format!("failed to read {}", source_root.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let file_name = entry.file_name();
        if should_skip_copy(&file_name) {
            continue;
        }

        let target_path = target_root.join(&file_name);
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if !target_path.exists() {
                fs::create_dir_all(&target_path)
                    .with_context(|| format!("failed to create {}", target_path.display()))?;
            }
            ensure_scaffold_layout(&source_path, &target_path)?;
        } else if !target_path.exists() {
            fs::copy(&source_path, &target_path).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    source_path.display(),
                    target_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn should_skip_copy(file_name: &std::ffi::OsStr) -> bool {
    matches!(
        file_name.to_str(),
        Some(".git")
            | Some(".venv")
            | Some("node_modules")
            | Some("target")
            | Some("dist")
            | Some("build")
            | Some("__pycache__")
    )
}

fn owned_paths() -> [&'static str; 9] {
    [
        "README.md",
        ".github/workflows/ci.yml",
        ".github/workflows/publish.yml",
        ".github/workflows/publish-npm.yml",
        "core-rs/Cargo.toml",
        "wrapper-js/package.json",
        "wrapper-py/pyproject.toml",
        "wrapper-js/README.md",
        "wrapper-py/README.md",
    ]
}

fn managed_files() -> Vec<ManagedFile> {
    vec![
        ManagedFile {
            path: "README.md",
            validate: validate_root_readme,
            sync: sync_root_readme,
        },
        ManagedFile {
            path: ".github/workflows/ci.yml",
            validate: validate_ci_workflow,
            sync: sync_ci_workflow,
        },
        ManagedFile {
            path: ".github/workflows/publish.yml",
            validate: validate_publish_workflow,
            sync: sync_publish_workflow,
        },
        ManagedFile {
            path: ".github/workflows/publish-npm.yml",
            validate: validate_publish_npm_workflow,
            sync: sync_publish_npm_workflow,
        },
        ManagedFile {
            path: "core-rs/Cargo.toml",
            validate: validate_cargo_toml,
            sync: sync_cargo_toml,
        },
        ManagedFile {
            path: "wrapper-js/package.json",
            validate: validate_package_json,
            sync: sync_package_json,
        },
        ManagedFile {
            path: "wrapper-py/pyproject.toml",
            validate: validate_pyproject,
            sync: sync_pyproject,
        },
        ManagedFile {
            path: "wrapper-js/README.md",
            validate: validate_js_readme,
            sync: sync_js_readme,
        },
        ManagedFile {
            path: "wrapper-py/README.md",
            validate: validate_py_readme,
            sync: sync_py_readme,
        },
    ]
}

fn validate_cargo_toml(config: &ToolConfig, content: &str) -> Result<Vec<ValidationIssue>> {
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

fn sync_cargo_toml(config: &ToolConfig, content: &str) -> Result<String> {
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

fn validate_package_json(config: &ToolConfig, content: &str) -> Result<Vec<ValidationIssue>> {
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
    Ok(issues)
}

fn sync_package_json(config: &ToolConfig, content: &str) -> Result<String> {
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
    let mut rendered = serde_json::to_string_pretty(&value)?;
    rendered.push('\n');
    Ok(rendered)
}

fn validate_pyproject(config: &ToolConfig, content: &str) -> Result<Vec<ValidationIssue>> {
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

fn sync_pyproject(config: &ToolConfig, content: &str) -> Result<String> {
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

fn validate_js_readme(config: &ToolConfig, content: &str) -> Result<Vec<ValidationIssue>> {
    validate_wrapper_readme("wrapper-js/README.md", "JavaScript", config, content)
}

fn sync_js_readme(config: &ToolConfig, _content: &str) -> Result<String> {
    Ok(render_wrapper_readme("JavaScript", config))
}

fn validate_py_readme(config: &ToolConfig, content: &str) -> Result<Vec<ValidationIssue>> {
    validate_wrapper_readme("wrapper-py/README.md", "Python", config, content)
}

fn sync_py_readme(config: &ToolConfig, _content: &str) -> Result<String> {
    Ok(render_wrapper_readme("Python", config))
}

fn validate_root_readme(config: &ToolConfig, content: &str) -> Result<Vec<ValidationIssue>> {
    let expected = render_root_readme_identity(config);
    let actual = extract_marked_section(content, README_IDENTITY_START, README_IDENTITY_END)?;
    if actual == expected {
        Ok(Vec::new())
    } else {
        Ok(vec![issue(
            "README.md",
            "readme.identity",
            "owned metadata differs from the canonical project identity",
            Some(expected),
            Some(actual),
        )])
    }
}

fn sync_root_readme(config: &ToolConfig, content: &str) -> Result<String> {
    replace_marked_section(
        content,
        README_IDENTITY_START,
        README_IDENTITY_END,
        &render_root_readme_identity(config),
    )
}

fn validate_ci_workflow(config: &ToolConfig, content: &str) -> Result<Vec<ValidationIssue>> {
    validate_workflow_name(
        ".github/workflows/ci.yml",
        &format!("{} CI", config.project.name),
        content,
    )
}

fn sync_ci_workflow(config: &ToolConfig, content: &str) -> Result<String> {
    sync_workflow_name(content, &format!("{} CI", config.project.name))
}

fn validate_publish_workflow(config: &ToolConfig, content: &str) -> Result<Vec<ValidationIssue>> {
    validate_workflow_name(
        ".github/workflows/publish.yml",
        &format!("{} publishing", config.project.name),
        content,
    )
}

fn sync_publish_workflow(config: &ToolConfig, content: &str) -> Result<String> {
    sync_workflow_name(content, &format!("{} publishing", config.project.name))
}

fn validate_publish_npm_workflow(
    config: &ToolConfig,
    content: &str,
) -> Result<Vec<ValidationIssue>> {
    validate_workflow_name(
        ".github/workflows/publish-npm.yml",
        &format!("{} publish-npm", config.project.name),
        content,
    )
}

fn sync_publish_npm_workflow(config: &ToolConfig, content: &str) -> Result<String> {
    sync_workflow_name(content, &format!("{} publish-npm", config.project.name))
}

fn validate_wrapper_readme(
    path: &str,
    language: &str,
    config: &ToolConfig,
    content: &str,
) -> Result<Vec<ValidationIssue>> {
    let expected = render_wrapper_readme(language, config);
    if content == expected {
        Ok(Vec::new())
    } else {
        Ok(vec![issue(
            path,
            "readme.identity",
            "owned metadata differs from the canonical project identity",
            Some(expected),
            Some(content.to_string()),
        )])
    }
}

fn render_wrapper_readme(_language: &str, config: &ToolConfig) -> String {
    let image_url = github_raw_url(
        &config.project.repository,
        "dev",
        "assets/illustrations/chestplate.svg",
    );
    format!(
        r#"# {name}

<p align="center">
  <img src="{image_url}" alt="{name} armor" width="320">
</p>

`{command}` helps you start and maintain a project that ships the same CLI through Rust, npm, and PyPI.

Use it to:

- create a new scaffolded project
- initialize an existing directory
- validate project identity and metadata
- keep owned files in sync

Common commands:

```bash
{command} version
{command} create <target>
{command} init --path <dir>
{command} validate
{command} sync --check
```

Learn more:

- [Main documentation](../docs/README.md)
- [Testing guide](../docs/testing.md)
- [Architecture](../docs/architecture.md)
"#,
        name = config.project.name,
        command = config.packages.command,
        image_url = image_url,
    )
}

fn render_root_readme_identity(config: &ToolConfig) -> String {
    format!(
        "# {}\n\n{}\n",
        config.project.name, config.project.description
    )
}

fn github_raw_url(repository: &str, branch: &str, path: &str) -> String {
    let trimmed = repository.trim_end_matches('/');
    if let Some(rest) = trimmed.strip_prefix("https://github.com/") {
        format!("https://raw.githubusercontent.com/{rest}/{branch}/{path}")
    } else {
        format!("{trimmed}/{path}")
    }
}

fn validate_workflow_name(
    path: &str,
    expected_name: &str,
    content: &str,
) -> Result<Vec<ValidationIssue>> {
    let expected = format!("name: {}\n", expected_name);
    let actual = extract_marked_section(content, WORKFLOW_NAME_START, WORKFLOW_NAME_END)?;
    if actual == expected {
        Ok(Vec::new())
    } else {
        Ok(vec![issue(
            path,
            "workflow.name",
            "owned metadata differs from the canonical project identity",
            Some(expected),
            Some(actual),
        )])
    }
}

fn sync_workflow_name(content: &str, expected_name: &str) -> Result<String> {
    replace_marked_section(
        content,
        WORKFLOW_NAME_START,
        WORKFLOW_NAME_END,
        &format!("name: {}\n", expected_name),
    )
}

fn issue(
    file: &str,
    code: &str,
    message: &str,
    expected: Option<String>,
    actual: Option<String>,
) -> ValidationIssue {
    ValidationIssue {
        code: code.to_string(),
        file: file.to_string(),
        message: message.to_string(),
        expected,
        actual,
    }
}

fn format_human_issues(header: &str, issues: &[ValidationIssue]) -> String {
    let mut grouped = BTreeMap::<&str, Vec<&ValidationIssue>>::new();
    for issue in issues {
        grouped.entry(&issue.file).or_default().push(issue);
    }

    let mut lines = vec![header.to_string()];
    for (file, file_issues) in grouped {
        lines.push(format!("- {}", file));
        for issue in file_issues {
            lines.push(format!("  [{}] {}", issue.code, issue.message));
            if let Some(expected) = &issue.expected {
                lines.push(format!("    expected: {}", summarize_value(expected)));
            }
            if let Some(actual) = &issue.actual {
                lines.push(format!("    actual:   {}", summarize_value(actual)));
            }
        }
    }
    lines.join("\n")
}

fn summarize_value(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "\"\"".to_string();
    }

    let flattened = trimmed.replace('\n', "\\n");
    if flattened.len() <= 80 {
        format!("{flattened:?}")
    } else {
        format!("{:?}…", &flattened[..80])
    }
}

fn extract_marked_section(content: &str, start: &str, end: &str) -> Result<String> {
    let start_index = content
        .find(start)
        .ok_or_else(|| anyhow!("missing marker {}", start))?;
    let section_start = start_index + start.len();
    let end_index = content[section_start..]
        .find(end)
        .map(|index| section_start + index)
        .ok_or_else(|| anyhow!("missing marker {}", end))?;
    Ok(content[section_start..end_index]
        .trim_matches('\n')
        .to_string()
        + "\n")
}

fn replace_marked_section(
    content: &str,
    start: &str,
    end: &str,
    replacement: &str,
) -> Result<String> {
    let start_index = content
        .find(start)
        .ok_or_else(|| anyhow!("missing marker {}", start))?;
    let section_start = start_index + start.len();
    let end_index = content[section_start..]
        .find(end)
        .map(|index| section_start + index)
        .ok_or_else(|| anyhow!("missing marker {}", end))?;

    let mut rendered = String::new();
    rendered.push_str(&content[..section_start]);
    rendered.push('\n');
    rendered.push_str(replacement.trim_end());
    rendered.push('\n');
    rendered.push_str(&content[end_index..]);
    Ok(rendered)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn version_output_serializes() {
        let json = serde_json::to_string(&VersionOutput {
            tool: "ossplate",
            version: "0.1.0",
        })
        .unwrap();
        assert_eq!(json, r#"{"tool":"ossplate","version":"0.1.0"}"#);
    }

    #[test]
    fn validate_detects_drift() {
        let root = make_fixture_root();
        fs::write(
            root.join("wrapper-js/package.json"),
            "{\n  \"name\": \"bad\",\n  \"description\": \"Build one project, ship it everywhere.\",\n  \"bin\": { \"ossplate\": \"bin/ossplate.js\" },\n  \"author\": \"Stef <stefdevscore@github.com>\",\n  \"license\": \"Unlicense\",\n  \"repository\": { \"url\": \"https://github.com/stefdevscore/ossplate\" }\n}\n",
        )
        .unwrap();
        let output = validate_repo(&root).unwrap();
        assert!(!output.ok);
        assert!(output
            .issues
            .iter()
            .any(|issue| issue.file == "wrapper-js/package.json"));
    }

    #[test]
    fn sync_check_detects_drift_and_sync_fixes_it() {
        let root = make_fixture_root();
        fs::write(
            root.join("wrapper-js/package.json"),
            "{\n  \"name\": \"bad\"\n}\n",
        )
        .unwrap();

        let error = sync_repo(&root, true).unwrap_err().to_string();
        assert!(error.contains("sync check failed"));
        sync_repo(&root, false).unwrap();
        assert!(sync_repo(&root, true).is_ok());
        assert!(validate_repo(&root).unwrap().ok);
    }

    #[test]
    fn human_issue_output_groups_by_file() {
        let rendered = format_human_issues(
            "validation failed:",
            &[
                issue(
                    "README.md",
                    "readme.identity",
                    "owned metadata differs",
                    Some("expected title".to_string()),
                    Some("actual title".to_string()),
                ),
                issue(
                    "wrapper-js/package.json",
                    "name",
                    "owned metadata differs",
                    Some("expected-name".to_string()),
                    Some("actual-name".to_string()),
                ),
            ],
        );

        assert!(rendered.contains("validation failed:"));
        assert!(rendered.contains("- README.md"));
        assert!(rendered.contains("- wrapper-js/package.json"));
        assert!(rendered.contains("expected: \"expected title\""));
        assert!(rendered.contains("actual:   \"actual-name\""));
    }

    #[test]
    fn parses_validate_subcommand() {
        let cli = Cli::try_parse_from(["ossplate", "validate", "--json"]).unwrap();
        match cli.command {
            Commands::Validate { json, .. } => assert!(json),
            _ => panic!("expected validate"),
        }
    }

    #[test]
    fn parses_create_with_identity_overrides() {
        let cli = Cli::try_parse_from([
            "ossplate",
            "create",
            "demo",
            "--name",
            "Demo Tool",
            "--command",
            "demo-tool",
        ])
        .unwrap();
        match cli.command {
            Commands::Create { target, overrides } => {
                assert_eq!(target, PathBuf::from("demo"));
                assert_eq!(overrides.name.as_deref(), Some("Demo Tool"));
                assert_eq!(overrides.command.as_deref(), Some("demo-tool"));
            }
            _ => panic!("expected create"),
        }
    }

    #[test]
    fn create_scaffolds_a_target_directory() {
        let source_root = make_fixture_root();
        let target = std::env::temp_dir().join("ossplate-create-target");
        if target.exists() {
            fs::remove_dir_all(&target).unwrap();
        }

        create_scaffold_from(&source_root, &target, &IdentityOverrides::default()).unwrap();
        assert!(target.join("ossplate.toml").exists());
        assert!(target.join("core-rs/Cargo.toml").exists());
        assert!(validate_repo(&target).unwrap().ok);

        fs::remove_dir_all(&target).unwrap();
    }

    #[test]
    fn init_hydrates_an_existing_directory() {
        let source_root = make_fixture_root();
        let target = std::env::temp_dir().join("ossplate-init-target");
        if target.exists() {
            fs::remove_dir_all(&target).unwrap();
        }
        fs::create_dir_all(target.join("core-rs")).unwrap();
        fs::write(
            target.join("ossplate.toml"),
            fs::read_to_string(source_root.join("ossplate.toml")).unwrap(),
        )
        .unwrap();
        fs::write(
            target.join("core-rs/Cargo.toml"),
            r#"[package]
name = "bad-core"
version = "0.1.5"
"#,
        )
        .unwrap();

        init_scaffold_from(&source_root, &target, &IdentityOverrides::default()).unwrap();
        assert!(target.join("wrapper-js/package.json").exists());
        assert!(target.join("wrapper-py/pyproject.toml").exists());
        assert!(validate_repo(&target).unwrap().ok);

        fs::remove_dir_all(&target).unwrap();
    }

    #[test]
    fn create_applies_identity_overrides_before_sync() {
        let source_root = make_fixture_root();
        let target = std::env::temp_dir().join("ossplate-create-with-overrides");
        if target.exists() {
            fs::remove_dir_all(&target).unwrap();
        }

        create_scaffold_from(
            &source_root,
            &target,
            &IdentityOverrides {
                name: Some("Demo Tool".to_string()),
                description: Some("A demo scaffold".to_string()),
                repository: Some("https://example.com/demo".to_string()),
                license: Some("Apache-2.0".to_string()),
                author_name: Some("Demo Dev".to_string()),
                author_email: Some("demo@example.com".to_string()),
                rust_crate: Some("demo-core".to_string()),
                npm_package: Some("demo-wrapper-js".to_string()),
                python_package: Some("demo-wrapper-py".to_string()),
                command: Some("demo-tool".to_string()),
            },
        )
        .unwrap();

        let config = load_config(&target).unwrap();
        assert_eq!(config.project.name, "Demo Tool");
        assert_eq!(config.packages.command, "demo-tool");
        assert!(validate_repo(&target).unwrap().ok);

        fs::remove_dir_all(&target).unwrap();
    }

    #[test]
    fn sync_preserves_unowned_root_readme_content() {
        let root = make_fixture_root();
        let original = fs::read_to_string(root.join("README.md")).unwrap();
        fs::write(
            root.join("README.md"),
            original.replace(
                "Build one project, ship it everywhere",
                "Changed identity text",
            ),
        )
        .unwrap();

        sync_repo(&root, false).unwrap();
        let synced = fs::read_to_string(root.join("README.md")).unwrap();
        assert!(synced.contains("## What This Tool Gives You"));
        assert!(synced.contains("Build one project, ship it everywhere"));
    }

    #[test]
    fn create_fails_when_scaffold_source_is_incomplete() {
        let source_root = std::env::temp_dir().join("ossplate-incomplete-source");
        if source_root.exists() {
            fs::remove_dir_all(&source_root).unwrap();
        }
        fs::create_dir_all(&source_root).unwrap();
        fs::write(
            source_root.join("ossplate.toml"),
            fs::read_to_string(make_fixture_root().join("ossplate.toml")).unwrap(),
        )
        .unwrap();

        let error = ensure_scaffold_source_root(&source_root).unwrap_err();
        assert!(error
            .to_string()
            .contains("require a full scaffold source checkout"));

        fs::remove_dir_all(&source_root).unwrap();
    }

    #[test]
    fn discover_template_root_honors_env_override() {
        let source_root = make_fixture_root();
        unsafe {
            std::env::set_var("OSSPLATE_TEMPLATE_ROOT", &source_root);
        }
        let discovered = discover_template_root().unwrap();
        unsafe {
            std::env::remove_var("OSSPLATE_TEMPLATE_ROOT");
        }
        assert_eq!(discovered, source_root);
    }

    fn make_fixture_root() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("ossplate-fixture-{unique}"));
        fs::create_dir_all(root.join(".github/workflows")).unwrap();
        fs::create_dir_all(root.join("core-rs")).unwrap();
        fs::create_dir_all(root.join("wrapper-js")).unwrap();
        fs::create_dir_all(root.join("wrapper-py")).unwrap();
        let config = r#"[project]
name = "Ossplate"
slug = "ossplate"
description = "Build one project, ship it everywhere."
repository = "https://github.com/stefdevscore/ossplate"
license = "Unlicense"

[author]
name = "Stef"
email = "stefdevscore@github.com"

[packages]
rust_crate = "ossplate"
npm_package = "ossplate"
python_package = "ossplate"
command = "ossplate"
"#;
        fs::write(
            root.join("ossplate.toml"),
            config.replace("slug = \"ossplate\"\n", ""),
        )
        .unwrap();
        fs::write(
            root.join("core-rs/Cargo.toml"),
            r#"[package]
name = "ossplate"
version = "0.1.5"
edition = "2021"
authors = ["Stef <stefdevscore@github.com>"]
description = "A practical baseline for shipping one project across Cargo, npm, and PyPI without starting from scratch every time."
license = "Unlicense"
readme = "../README.md"
repository = "https://github.com/stefdevscore/ossplate"
homepage = "https://github.com/stefdevscore/ossplate"
"#,
        )
        .unwrap();
        fs::write(
            root.join("wrapper-js/package.json"),
            "{\n  \"name\": \"ossplate\",\n  \"description\": \"Build one project, ship it everywhere.\",\n  \"bin\": { \"ossplate\": \"bin/ossplate.js\" },\n  \"author\": \"Stef <stefdevscore@github.com>\",\n  \"license\": \"Unlicense\",\n  \"repository\": { \"url\": \"https://github.com/stefdevscore/ossplate\" }\n}\n",
        )
        .unwrap();
        fs::write(
            root.join("wrapper-py/pyproject.toml"),
            r#"[project]
name = "ossplate"
description = "Build one project, ship it everywhere."
license = { text = "Unlicense" }
authors = [
  { name = "Stef", email = "stefdevscore@github.com" }
]

[project.urls]
Homepage = "https://github.com/stefdevscore/ossplate"
Repository = "https://github.com/stefdevscore/ossplate"

[project.scripts]
ossplate = "ossplate.cli:main"
"#,
        )
        .unwrap();
        fs::write(
            root.join(".github/workflows/ci.yml"),
            format!(
                "{start}\nname: Ossplate CI\n{end}\n\non:\n  push:\n    branches:\n      - main\n",
                start = WORKFLOW_NAME_START,
                end = WORKFLOW_NAME_END
            ),
        )
        .unwrap();
        fs::write(
            root.join(".github/workflows/publish.yml"),
            format!(
                "{start}\nname: Ossplate publishing\n{end}\n\non:\n  workflow_dispatch:\n",
                start = WORKFLOW_NAME_START,
                end = WORKFLOW_NAME_END
            ),
        )
        .unwrap();
        fs::write(
            root.join(".github/workflows/publish-npm.yml"),
            format!(
                "{start}\nname: Ossplate publish-npm\n{end}\n\non:\n  workflow_dispatch:\n",
                start = WORKFLOW_NAME_START,
                end = WORKFLOW_NAME_END
            ),
        )
        .unwrap();
        fs::write(
            root.join("README.md"),
            format!(
                "{start}\n{body}{end}\n\n## What This Tool Gives You\n\n- a canonical Rust CLI in [`core-rs/`](./core-rs)\n",
                start = README_IDENTITY_START,
                body = render_root_readme_identity(&load_config(&root).unwrap()),
                end = README_IDENTITY_END
            ),
        )
        .unwrap();
        fs::write(
            root.join("wrapper-js/README.md"),
            render_wrapper_readme("JavaScript", &load_config(&root).unwrap()),
        )
        .unwrap();
        fs::write(
            root.join("wrapper-py/README.md"),
            render_wrapper_readme("Python", &load_config(&root).unwrap()),
        )
        .unwrap();
        root
    }
}
