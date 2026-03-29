use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

mod release;
mod scaffold;
mod sync;

use release::{publish_repo, PublishRegistry};
use scaffold::{create_scaffold, init_scaffold};
use sync::{format_human_issues, sync_repo, validate_repo};

#[cfg(test)]
pub(crate) use scaffold::{
    create_scaffold_from, discover_template_root, ensure_scaffold_source_root, init_scaffold_from,
};

#[cfg(test)]
pub(crate) use sync::{
    github_blob_url, github_raw_url, issue, render_root_readme_identity, render_wrapper_readme,
    README_IDENTITY_END, README_IDENTITY_START, WORKFLOW_NAME_END, WORKFLOW_NAME_START,
};

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
    /// Publish the current checked-out version from source without mutating git state
    Publish {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, value_enum, default_value_t = PublishRegistry::All)]
        registry: PublishRegistry,
        #[arg(long)]
        skip_existing: bool,
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
        Commands::Publish {
            path,
            dry_run,
            registry,
            skip_existing,
        } => publish_repo(&path, dry_run, registry, skip_existing),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_path(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{unique}"))
    }

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
            "{\n  \"name\": \"bad\",\n  \"version\": \"0.1.19\",\n  \"optionalDependencies\": {}\n}\n",
        )
        .unwrap();

        let error = sync_repo(&root, true).unwrap_err().to_string();
        assert!(error.contains("sync check failed"));
        sync_repo(&root, false).unwrap();
        assert!(sync_repo(&root, true).is_ok());
        assert!(validate_repo(&root).unwrap().ok);
    }

    #[test]
    fn sync_owns_runtime_package_metadata() {
        let root = make_fixture_root();
        fs::write(
            root.join("wrapper-js/platform-packages/ossplate-darwin-arm64/package.json"),
            "{\n  \"name\": \"bad-runtime\"\n}\n",
        )
        .unwrap();

        let error = sync_repo(&root, true).unwrap_err().to_string();
        assert!(error.contains("sync check failed"));
        sync_repo(&root, false).unwrap();
        let synced = fs::read_to_string(
            root.join("wrapper-js/platform-packages/ossplate-darwin-arm64/package.json"),
        )
        .unwrap();
        assert!(synced.contains("\"name\": \"ossplate-darwin-arm64\""));
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
    fn parses_publish_with_flags() {
        let cli = Cli::try_parse_from([
            "ossplate",
            "publish",
            "--path",
            "demo",
            "--dry-run",
            "--registry",
            "pypi",
            "--skip-existing",
        ])
        .unwrap();
        match cli.command {
            Commands::Publish {
                path,
                dry_run,
                registry,
                skip_existing,
            } => {
                assert_eq!(path, PathBuf::from("demo"));
                assert!(dry_run);
                assert_eq!(registry, PublishRegistry::Pypi);
                assert!(skip_existing);
            }
            _ => panic!("expected publish"),
        }
    }

    #[test]
    fn create_scaffolds_a_target_directory() {
        let source_root = make_fixture_root();
        let target = unique_temp_path("ossplate-create-target");
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
        let target = unique_temp_path("ossplate-init-target");
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
version = "0.1.22"
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
        let target = unique_temp_path("ossplate-create-with-overrides");
        if target.exists() {
            fs::remove_dir_all(&target).unwrap();
        }

        create_scaffold_from(
            &source_root,
            &target,
            &IdentityOverrides {
                name: Some("Demo Tool".to_string()),
                description: Some("A demo scaffold".to_string()),
                repository: Some("https://github.com/example/demo".to_string()),
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
        let wrapper_package: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(target.join("wrapper-js/package.json")).unwrap(),
        )
        .unwrap();
        let mut actual_runtime_dependencies = wrapper_package["optionalDependencies"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        actual_runtime_dependencies.sort();
        assert_eq!(
            actual_runtime_dependencies,
            vec![
                "demo-wrapper-js-darwin-arm64".to_string(),
                "demo-wrapper-js-darwin-x64".to_string(),
                "demo-wrapper-js-linux-x64".to_string(),
                "demo-wrapper-js-windows-x64".to_string(),
            ]
        );
        let runtime_package: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(
                target.join("wrapper-js/platform-packages/ossplate-darwin-arm64/package.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            runtime_package["name"].as_str().unwrap(),
            "demo-wrapper-js-darwin-arm64"
        );
        assert!(validate_repo(&target).unwrap().ok);

        fs::remove_dir_all(&target).unwrap();
    }

    #[test]
    fn create_fails_when_target_directory_is_not_empty() {
        let source_root = make_fixture_root();
        let target = unique_temp_path("ossplate-create-non-empty");
        if target.exists() {
            fs::remove_dir_all(&target).unwrap();
        }
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("placeholder.txt"), "occupied").unwrap();

        let error = create_scaffold_from(&source_root, &target, &IdentityOverrides::default())
            .unwrap_err()
            .to_string();
        assert!(error.contains("target directory is not empty"));

        fs::remove_dir_all(&target).unwrap();
    }

    #[test]
    fn create_fails_when_target_is_inside_source_tree() {
        let source_root = make_fixture_root();
        let target = source_root.join("nested-output");
        if target.exists() {
            fs::remove_dir_all(&target).unwrap();
        }

        let error = create_scaffold_from(&source_root, &target, &IdentityOverrides::default())
            .unwrap_err()
            .to_string();
        assert!(error.contains("target directory must not be inside the source template tree"));

        fs::remove_dir_all(&source_root).unwrap();
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
    fn github_link_helpers_render_absolute_main_urls() {
        let repository = "https://github.com/stefdevscore/ossplate";
        assert_eq!(
            github_blob_url(repository, "main", "docs/README.md").unwrap(),
            "https://github.com/stefdevscore/ossplate/blob/main/docs/README.md"
        );
        assert_eq!(
            github_raw_url(repository, "main", "assets/illustrations/chestplate.svg").unwrap(),
            "https://raw.githubusercontent.com/stefdevscore/ossplate/main/assets/illustrations/chestplate.svg"
        );
    }

    #[test]
    fn rendered_wrapper_readmes_use_absolute_doc_links() {
        let config = load_config(&make_fixture_root()).unwrap();
        let rendered = render_wrapper_readme("Python", &config);
        assert!(
            rendered.contains("https://github.com/stefdevscore/ossplate/blob/main/docs/README.md")
        );
        assert!(!rendered.contains("../docs/README.md"));
        assert!(!rendered.contains("../docs/testing.md"));
        assert!(!rendered.contains("../docs/architecture.md"));
    }

    #[test]
    fn validate_accepts_crlf_owned_text_surfaces() {
        let root = make_fixture_root();
        sync_repo(&root, false).unwrap();
        for path in [
            "README.md",
            ".github/workflows/ci.yml",
            ".github/workflows/publish.yml",
            ".github/workflows/publish-npm.yml",
            "wrapper-js/README.md",
            "wrapper-py/README.md",
        ] {
            let content = fs::read_to_string(root.join(path)).unwrap();
            fs::write(root.join(path), content.replace('\n', "\r\n")).unwrap();
        }

        let output = validate_repo(&root).unwrap();
        assert!(output.ok, "{:?}", output.issues);
    }

    #[test]
    fn sync_preserves_crlf_when_rewriting_marked_sections() {
        let root = make_fixture_root();
        let readme_path = root.join("README.md");
        let workflow_path = root.join(".github/workflows/ci.yml");
        let readme = fs::read_to_string(&readme_path)
            .unwrap()
            .replace('\n', "\r\n");
        let workflow = fs::read_to_string(&workflow_path)
            .unwrap()
            .replace('\n', "\r\n");
        fs::write(
            &readme_path,
            readme.replace("Build one project, ship it everywhere.", "Drifted text"),
        )
        .unwrap();
        fs::write(
            &workflow_path,
            workflow.replace("name: Ossplate CI", "name: Drifted CI"),
        )
        .unwrap();

        sync_repo(&root, false).unwrap();

        let synced_readme = fs::read_to_string(&readme_path).unwrap();
        let synced_workflow = fs::read_to_string(&workflow_path).unwrap();
        assert!(synced_readme.contains("\r\n"));
        assert!(synced_workflow.contains("\r\n"));
        assert!(validate_repo(&root).unwrap().ok);
    }

    #[test]
    fn create_fails_when_scaffold_source_is_incomplete() {
        let source_root = unique_temp_path("ossplate-incomplete-source");
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
        fs::create_dir_all(root.join("wrapper-js/platform-packages/ossplate-darwin-arm64"))
            .unwrap();
        fs::create_dir_all(root.join("wrapper-js/platform-packages/ossplate-darwin-x64")).unwrap();
        fs::create_dir_all(root.join("wrapper-js/platform-packages/ossplate-linux-x64")).unwrap();
        fs::create_dir_all(root.join("wrapper-js/platform-packages/ossplate-win32-x64")).unwrap();
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
version = "0.1.22"
edition = "2021"
authors = ["Stef <stefdevscore@github.com>"]
description = "Build one project, ship it everywhere."
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
        for (target, os, cpu) in [
            ("darwin-arm64", "darwin", "arm64"),
            ("darwin-x64", "darwin", "x64"),
            ("linux-x64", "linux", "x64"),
            ("win32-x64", "win32", "x64"),
        ] {
            let package_name = if target == "win32-x64" {
                "ossplate-windows-x64".to_string()
            } else {
                format!("ossplate-{target}")
            };
            let package_folder = format!("ossplate-{target}");
            let description = format!("Platform runtime package for ossplate on {target}.");
            let directory = format!("wrapper-js/platform-packages/{package_folder}");
            let manifest = format!(
                "{{\n  \"name\": \"{package_name}\",\n  \"description\": \"{description}\",\n  \"license\": \"Unlicense\",\n  \"repository\": {{\n    \"type\": \"git\",\n    \"url\": \"https://github.com/stefdevscore/ossplate\",\n    \"directory\": \"{directory}\"\n  }},\n  \"os\": [\"{os}\"],\n  \"cpu\": [\"{cpu}\"]\n}}\n"
            );
            fs::write(
                root.join(format!(
                    "wrapper-js/platform-packages/{package_folder}/package.json"
                )),
                manifest,
            )
            .unwrap();
        }
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
