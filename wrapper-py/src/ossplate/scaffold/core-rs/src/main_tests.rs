use crate::config::IdentityOverrides;
use crate::output::VersionOutput;
use crate::release::PublishRegistry;
use crate::scaffold::{
    create_scaffold_from, discover_template_root, ensure_scaffold_source_root, init_scaffold_from,
};
use crate::sync::format_human_issues;
use crate::sync::{
    github_blob_url, github_raw_url, issue, render_wrapper_readme, sync_repo, validate_repo,
};
use crate::test_support::{fs, load_config, Path};
use crate::{Cli, Commands};
use clap::Parser;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_temp_path(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("{prefix}-{unique}-{counter}"))
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
    let config = load_config(&root).unwrap();
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
    assert!(synced.contains(&format!(
        "\"name\": \"{}-darwin-arm64\"",
        config.packages.npm_package
    )));
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
    let source_root = make_source_checkout_root();
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
    let source_root = make_source_checkout_root();
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
    let source_root = make_source_checkout_root();
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
    let wrapper_package: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(target.join("wrapper-js/package.json")).unwrap())
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
fn create_with_non_default_package_identity_is_valid_immediately() {
    let source_root = make_source_checkout_root();
    let target = unique_temp_path("ossplate-bootstrap-agentcode");
    if target.exists() {
        fs::remove_dir_all(&target).unwrap();
    }

    create_scaffold_from(
        &source_root,
        &target,
        &IdentityOverrides {
            name: Some("Agentcode".to_string()),
            description: Some(
                "Build and ship the agentcode CLI through Rust, npm, and PyPI.".to_string(),
            ),
            repository: Some("https://github.com/stefdevscore/agentcode".to_string()),
            license: Some("Apache-2.0".to_string()),
            author_name: Some("Azk".to_string()),
            author_email: Some("azk@example.com".to_string()),
            rust_crate: None,
            npm_package: None,
            python_package: None,
            command: Some("agentcode".to_string()),
        },
    )
    .unwrap();

    assert!(validate_repo(&target).unwrap().ok);
    assert!(sync_repo(&target, true).is_ok());

    let cargo_toml = fs::read_to_string(target.join("core-rs/Cargo.toml")).unwrap();
    assert!(cargo_toml.contains("name = \"agentcode\""));
    assert!(cargo_toml.contains("default-run = \"agentcode\""));
    assert!(cargo_toml.contains("[[bin]]"));

    let wrapper_package: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(target.join("wrapper-js/package.json")).unwrap())
            .unwrap();
    assert_eq!(wrapper_package["name"], "agentcode");
    assert_eq!(wrapper_package["bin"]["agentcode"], "bin/agentcode.js");
    assert!(target.join("wrapper-js/bin/agentcode.js").exists());
    assert!(!target.join("wrapper-js/bin/ossplate.js").exists());

    let runtime_package: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(
            target.join("wrapper-js/platform-packages/ossplate-darwin-arm64/package.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(runtime_package["name"], "agentcode-darwin-arm64");
    let runtime_targets: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(target.join("runtime-targets.json")).unwrap())
            .unwrap();
    assert_eq!(runtime_targets["targets"][0]["binary"], "agentcode");
    assert_eq!(runtime_targets["targets"][3]["binary"], "agentcode.exe");

    let pyproject = fs::read_to_string(target.join("wrapper-py/pyproject.toml")).unwrap();
    assert!(pyproject.contains("name = \"agentcode\""));
    assert!(pyproject.contains("agentcode.cli:main"));
    assert!(pyproject.contains("packages = [\"src/agentcode\"]"));
    assert!(target.join("wrapper-py/src/agentcode/cli.py").exists());
    assert!(!target.join("wrapper-py/src/ossplate/cli.py").exists());

    fs::remove_dir_all(&target).unwrap();
}

#[test]
fn create_fails_when_target_directory_is_not_empty() {
    let source_root = make_source_checkout_root();
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
    let source_root = make_source_checkout_root();
    let target = source_root.join("nested-output");
    if target.exists() {
        fs::remove_dir_all(&target).unwrap();
    }

    let error = create_scaffold_from(&source_root, &target, &IdentityOverrides::default())
        .unwrap_err()
        .to_string();
    assert!(error.contains("target directory must not be inside the source template tree"));

    fs::remove_dir_all(&target).unwrap();
}

#[test]
fn sync_preserves_unowned_root_readme_content() {
    let root = make_fixture_root();
    let config = load_config(&root).unwrap();
    let original = fs::read_to_string(root.join("README.md")).unwrap();
    fs::write(
        root.join("README.md"),
        original.replace(&config.project.description, "Changed identity text"),
    )
    .unwrap();

    sync_repo(&root, false).unwrap();
    let synced = fs::read_to_string(root.join("README.md")).unwrap();
    assert!(synced.contains("## What It Does"));
    assert!(synced.contains(&config.project.description));
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
    assert!(rendered
        .contains(&github_blob_url(&config.project.repository, "main", "docs/README.md").unwrap()));
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
        ".github/workflows/live-e2e-published.yml",
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
    let source_root = make_source_checkout_root();
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
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    copy_required_paths_from_manifest(&repo_root, &root);
    root
}

fn make_source_checkout_root() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("ossplate-source-fixture-{unique}"));
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    copy_required_paths_from_manifest(&repo_root, &root);
    root
}

fn copy_required_paths_from_manifest(source_root: &Path, target_root: &Path) {
    fs::create_dir_all(target_root).unwrap();
    fs::copy(
        source_root.join("scaffold-payload.json"),
        target_root.join("scaffold-payload.json"),
    )
    .unwrap();
    let manifest: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(source_root.join("scaffold-payload.json")).unwrap(),
    )
    .unwrap();
    for relative_path in manifest["requiredPaths"].as_array().unwrap() {
        let relative_path = relative_path.as_str().unwrap();
        let source_path = source_root.join(relative_path);
        let target_path = target_root.join(relative_path);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        if source_path.is_dir() {
            copy_fixture_tree(&source_path, &target_path);
        } else {
            fs::copy(&source_path, &target_path).unwrap();
        }
    }
}

fn copy_fixture_tree(source_root: &Path, target_root: &Path) {
    fs::create_dir_all(target_root).unwrap();
    for entry in fs::read_dir(source_root).unwrap() {
        let entry = entry.unwrap();
        let source_path = entry.path();
        let target_path = target_root.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_fixture_tree(&source_path, &target_path);
        } else {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::copy(&source_path, &target_path).unwrap();
        }
    }
}
