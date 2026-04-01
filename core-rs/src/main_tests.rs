use crate::config::{
    generated_project_description, is_template_project, latest_scaffold_version, write_config,
    IdentityOverrides, GENERATED_AUTHOR_EMAIL_PLACEHOLDER, GENERATED_AUTHOR_NAME_PLACEHOLDER,
    GENERATED_REPOSITORY_PLACEHOLDER,
};
use crate::embedded_template::{embedded_template_contains, materialize_embedded_template_root};
use crate::output::VersionOutput;
use crate::release::{render_publish_plan, PublishRegistry};
use crate::scaffold::{
    create_scaffold_from, create_scaffold_json, discover_template_root,
    ensure_scaffold_source_root, init_scaffold_from, init_scaffold_json,
};
use crate::sync::format_human_issues;
use crate::sync::{
    github_blob_url, github_raw_url, inspect_repo_json, issue, render_wrapper_readme,
    sync_apply_json, sync_check_json, sync_plan_json, sync_repo, validate_repo,
};
use crate::test_support::{fs, load_config, Path};
use crate::upgrade::{inspect_compatibility, upgrade_apply_json, upgrade_plan_json, Compatibility};
use crate::upgrade_catalog::{authored_versions, latest_authored_version};
use crate::verify::VerifyStepResult;
use crate::{Cli, Commands};
use clap::Parser;
use std::path::PathBuf;
use std::process::Command;
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
        "--json",
        "--name",
        "Demo Tool",
        "--command",
        "demo-tool",
    ])
    .unwrap();
    match cli.command {
        Commands::Create {
            target,
            json,
            overrides,
        } => {
            assert_eq!(target, PathBuf::from("demo"));
            assert!(json);
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
        "--plan",
        "--json",
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
            plan,
            json,
        } => {
            assert_eq!(path, PathBuf::from("demo"));
            assert!(dry_run);
            assert_eq!(registry, PublishRegistry::Pypi);
            assert!(skip_existing);
            assert!(plan);
            assert!(json);
        }
        _ => panic!("expected publish"),
    }
}

#[test]
fn parses_verify_with_json() {
    let cli = Cli::try_parse_from(["ossplate", "verify", "--path", "demo", "--json"]).unwrap();
    match cli.command {
        Commands::Verify { path, json } => {
            assert_eq!(path, PathBuf::from("demo"));
            assert!(json);
        }
        _ => panic!("expected verify"),
    }
}

#[test]
fn parses_upgrade_with_plan_and_json() {
    let cli =
        Cli::try_parse_from(["ossplate", "upgrade", "--path", "demo", "--plan", "--json"]).unwrap();
    match cli.command {
        Commands::Upgrade { path, plan, json } => {
            assert_eq!(path, PathBuf::from("demo"));
            assert!(plan);
            assert!(json);
        }
        _ => panic!("expected upgrade"),
    }
}

#[test]
fn sync_check_json_returns_ok_on_clean_repo() {
    let root = make_fixture_root();
    let output: serde_json::Value = serde_json::from_str(&sync_check_json(&root).unwrap()).unwrap();
    assert_eq!(output["ok"], true);
    assert_eq!(output["mode"], "check");
    assert_eq!(output["issues"], serde_json::json!([]));
    assert_eq!(output["changes"], serde_json::json!([]));
}

#[test]
fn sync_check_json_returns_structured_drift() {
    let root = make_fixture_root();
    fs::write(
        root.join("wrapper-js/package.json"),
        "{\n  \"name\": \"bad\",\n  \"version\": \"0.1.19\",\n  \"optionalDependencies\": {}\n}\n",
    )
    .unwrap();

    let output: serde_json::Value = serde_json::from_str(&sync_check_json(&root).unwrap()).unwrap();
    assert_eq!(output["ok"], false);
    assert_eq!(output["mode"], "check");
    assert!(output["issues"].as_array().unwrap().len() > 0);
    assert!(output["changes"].as_array().unwrap().len() > 0);
    assert!(output["changes"][0].get("synced").is_none());
}

#[test]
fn sync_plan_json_includes_synced_content_and_does_not_mutate() {
    let root = make_fixture_root();
    let original = fs::read_to_string(root.join("wrapper-js/package.json")).unwrap();
    fs::write(
        root.join("wrapper-js/package.json"),
        "{\n  \"name\": \"bad\",\n  \"version\": \"0.1.19\",\n  \"optionalDependencies\": {}\n}\n",
    )
    .unwrap();

    let output: serde_json::Value = serde_json::from_str(&sync_plan_json(&root).unwrap()).unwrap();
    assert_eq!(output["ok"], false);
    assert_eq!(output["mode"], "plan");
    let changes = output["changes"].as_array().unwrap();
    assert!(changes
        .iter()
        .any(|change| change["file"] == "wrapper-js/package.json"));
    assert!(changes.iter().all(|change| change["synced"].is_string()));

    let current = fs::read_to_string(root.join("wrapper-js/package.json")).unwrap();
    assert_ne!(current, original);
    assert!(current.contains("\"name\": \"bad\""));
}

#[test]
fn sync_apply_json_mutates_and_reports_changed_files() {
    let root = make_fixture_root();
    fs::write(
        root.join("wrapper-js/package.json"),
        "{\n  \"name\": \"bad\",\n  \"version\": \"0.1.19\",\n  \"optionalDependencies\": {}\n}\n",
    )
    .unwrap();

    let planned: serde_json::Value = serde_json::from_str(&sync_plan_json(&root).unwrap()).unwrap();
    let applied: serde_json::Value =
        serde_json::from_str(&sync_apply_json(&root).unwrap()).unwrap();

    assert_eq!(applied["mode"], "apply");
    assert_eq!(applied["issues"], planned["issues"]);
    assert_eq!(applied["changes"], planned["changes"]);
    assert!(validate_repo(&root).unwrap().ok);
}

#[test]
fn render_verify_output_serializes_step_results() {
    let json = crate::output::render_verify_output(vec![
        VerifyStepResult {
            name: "tool:validate".into(),
            ok: true,
            exit_code: 0,
            stdout: "{\"ok\":true}".into(),
            stderr: String::new(),
            skipped: false,
            reason: None,
        },
        VerifyStepResult {
            name: "js:test".into(),
            ok: true,
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            skipped: true,
            reason: Some("not published".into()),
        },
    ])
    .unwrap();
    let output: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(output["ok"], true);
    assert_eq!(output["steps"][0]["name"], "tool:validate");
    assert_eq!(output["steps"][0]["exitCode"], 0);
    assert_eq!(output["steps"][1]["skipped"], true);
    assert_eq!(output["steps"][1]["reason"], "not published");
}

#[test]
fn verify_json_failure_still_renders_machine_readable_output() {
    let output = crate::output::VerifyOutput {
        ok: false,
        steps: vec![VerifyStepResult {
            name: "rust:test".into(),
            ok: false,
            exit_code: 1,
            stdout: String::new(),
            stderr: "boom".into(),
            skipped: false,
            reason: None,
        }],
    };
    let rendered = crate::output::render_verify_output(output.steps).unwrap();
    let value: serde_json::Value = serde_json::from_str(&rendered).unwrap();
    assert_eq!(value["ok"], false);
    assert_eq!(value["steps"][0]["name"], "rust:test");
    assert_eq!(value["steps"][0]["stderr"], "boom");
}

#[test]
fn inspect_json_returns_config_and_contracts() {
    let root = make_source_checkout_root();
    let output: serde_json::Value =
        serde_json::from_str(&inspect_repo_json(&root).unwrap()).unwrap();
    assert_eq!(output["config"]["project"]["name"], "Ossplate");
    assert_eq!(output["scaffoldVersion"], 3);
    assert_eq!(output["latestScaffoldVersion"], 3);
    assert_eq!(output["compatibility"], "current");
    assert!(output["managedFiles"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry == "core-rs/Cargo.toml"));
    assert!(output["runtimeTargets"]["targets"].is_array());
    assert!(output["scaffoldPayload"]["requiredPaths"].is_array());
    assert!(output["sourceCheckout"]["requiredPaths"].is_array());
    assert_eq!(
        output["derived"]["paths"]["jsWrapperLauncher"],
        "wrapper-js/bin/ossplate.js"
    );
    assert_eq!(output["derived"]["paths"]["pythonModule"], "ossplate");
    assert!(output["derived"]["runtimePackages"].is_array());
    assert_eq!(
        output["derived"]["runtimePackages"][0]["folder"],
        "wrapper-js/platform-packages/ossplate-darwin-arm64"
    );
}

#[test]
fn generated_descendants_include_current_scaffold_version() {
    let source_root = repo_root();
    let target = unique_temp_path("ossplate-generated-version");
    create_scaffold_from(&source_root, &target, &IdentityOverrides::default()).unwrap();
    let config = load_config(&target).unwrap();
    assert_eq!(config.template.scaffold_version, Some(3));
    fs::remove_dir_all(target).unwrap();
}

#[test]
fn authored_versions_stay_aligned_with_current_scaffold_contract() {
    let authored = authored_versions();
    assert!(!authored.is_empty());
    assert_eq!(latest_authored_version(), latest_scaffold_version());

    let latest = authored
        .iter()
        .find(|spec| spec.version == latest_scaffold_version())
        .expect("current scaffold version must have an authored spec");
    assert_eq!(
        latest.fingerprint.required_paths,
        crate::scaffold_manifest::required_source_paths()
    );
    assert!(
        latest.fingerprint.forbidden_paths.is_empty(),
        "current scaffold fingerprint should not rely on forbidden-path exceptions"
    );

    let root_scaffold_payload =
        crate::scaffold_manifest::read_path_manifest(&repo_root().join("scaffold-payload.json"))
            .unwrap();
    let core_scaffold_payload = crate::scaffold_manifest::read_path_manifest(
        &repo_root().join("core-rs/scaffold-payload.json"),
    )
    .unwrap();
    assert_eq!(root_scaffold_payload, core_scaffold_payload);
}

#[test]
fn authored_versions_form_a_contiguous_upgrade_chain() {
    let authored = authored_versions();
    let versions = authored.iter().map(|spec| spec.version).collect::<Vec<_>>();
    assert_eq!(versions, vec![1, 2, 3]);

    for window in authored.windows(2) {
        let previous = &window[0];
        let current = &window[1];
        let migration = current
            .migration_from_previous
            .as_ref()
            .expect("every non-initial scaffold version must define a migration");
        assert_eq!(migration.from_version, previous.version);
        assert_eq!(migration.to_version, current.version);
    }

    assert!(
        authored[0].migration_from_previous.is_none(),
        "initial scaffold version must not define a previous migration"
    );
}

#[test]
fn inspect_json_reports_upgrade_supported_for_previous_descendant() {
    let root = make_previous_version_descendant();
    let output: serde_json::Value =
        serde_json::from_str(&inspect_repo_json(&root).unwrap()).unwrap();
    assert_eq!(output["scaffoldVersion"], 2);
    assert_eq!(output["latestScaffoldVersion"], 3);
    assert_eq!(output["compatibility"], "upgrade_supported");
    assert_eq!(output["recommendedAction"], "upgrade");
    assert_eq!(output["upgradePath"], serde_json::json!(["2->3"]));
    assert!(output.get("blockingReason").is_none());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn inspect_json_reports_recreate_for_older_descendant() {
    let root = make_previous_version_descendant();
    let mut config = load_config(&root).unwrap();
    config.template.scaffold_version = Some(0);
    write_config(&root, &config).unwrap();
    let output: serde_json::Value =
        serde_json::from_str(&inspect_repo_json(&root).unwrap()).unwrap();
    assert_eq!(output["compatibility"], "recreate_recommended");
    assert_eq!(output["recommendedAction"], "recreate");
    assert_eq!(output["upgradePath"], serde_json::json!([]));
    assert_eq!(
        output["blockingReason"],
        "upgrade path is unavailable from scaffold version 0 to 3"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn inspect_json_reports_unsupported_for_damaged_descendant() {
    let root = make_previous_version_descendant();
    fs::remove_file(root.join("core-rs/src/main.rs")).unwrap();
    let output: serde_json::Value =
        serde_json::from_str(&inspect_repo_json(&root).unwrap()).unwrap();
    assert_eq!(output["compatibility"], "unsupported");
    assert_eq!(output["recommendedAction"], "stop");
    assert_eq!(output["upgradePath"], serde_json::json!([]));
    assert!(output["blockingReason"]
        .as_str()
        .unwrap()
        .contains("does not match"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn inspect_json_maps_unversioned_exact_match_descendant_to_known_upgrade_path() {
    let root = make_previous_version_descendant();
    let mut config = load_config(&root).unwrap();
    config.template.scaffold_version = None;
    write_config(&root, &config).unwrap();
    let output: serde_json::Value =
        serde_json::from_str(&inspect_repo_json(&root).unwrap()).unwrap();
    assert_eq!(output["scaffoldVersion"], 2);
    assert_eq!(output["compatibility"], "upgrade_supported");
    assert_eq!(output["upgradePath"], serde_json::json!(["2->3"]));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn inspect_json_rejects_unversioned_descendant_with_drifted_manifest_contract() {
    let root = make_previous_version_descendant();
    let mut config = load_config(&root).unwrap();
    config.template.scaffold_version = None;
    write_config(&root, &config).unwrap();
    remove_required_path_from_json(&root.join("source-checkout.json"), "scripts/package-js.mjs");
    let output: serde_json::Value =
        serde_json::from_str(&inspect_repo_json(&root).unwrap()).unwrap();
    assert_eq!(output["compatibility"], "recreate_recommended");
    assert_eq!(output["recommendedAction"], "recreate");
    assert_eq!(output["upgradePath"], serde_json::json!([]));
    assert!(output["blockingReason"]
        .as_str()
        .unwrap()
        .contains("does not exactly match"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn upgrade_plan_json_is_stable_and_non_mutating() {
    let root = make_version_1_descendant();
    let before = fs::read_to_string(root.join("ossplate.toml")).unwrap();
    let output: serde_json::Value =
        serde_json::from_str(&upgrade_plan_json(&root).unwrap()).unwrap();
    assert_eq!(output["ok"], true);
    assert_eq!(output["apply"], false);
    assert_eq!(output["fromVersion"], 1);
    assert_eq!(output["toVersion"], 3);
    assert_eq!(output["compatibility"], "upgrade_supported");
    assert_eq!(output["canApply"], true);
    assert_eq!(output["upgradePath"], serde_json::json!(["1->2", "2->3"]));
    assert!(output["changedFiles"].as_array().unwrap().len() > 0);
    assert_eq!(output["stepPlans"].as_array().unwrap().len(), 2);
    assert_eq!(output["stepPlans"][0]["step"], "1->2");
    assert_eq!(output["stepPlans"][1]["step"], "2->3");
    assert!(output["stepPlans"][0]["changedFiles"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("core-rs/src/upgrade.rs")));
    assert!(output["stepPlans"][1]["changedFiles"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("core-rs/src/upgrade_catalog.rs")));
    let after = fs::read_to_string(root.join("ossplate.toml")).unwrap();
    assert_eq!(before, after);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn upgrade_apply_updates_previous_descendant_and_reports_changes() {
    let root = make_previous_version_descendant();
    let output: serde_json::Value =
        serde_json::from_str(&upgrade_apply_json(&root).unwrap()).unwrap();
    assert_eq!(output["ok"], true);
    assert_eq!(output["apply"], true);
    assert_eq!(output["fromVersion"], 2);
    assert_eq!(output["toVersion"], 3);
    assert_eq!(output["upgradePath"], serde_json::json!(["2->3"]));
    assert!(root.join("core-rs/src/upgrade_catalog.rs").exists());
    let config = load_config(&root).unwrap();
    assert_eq!(config.template.scaffold_version, Some(3));
    assert!(validate_repo(&root).unwrap().ok);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn upgrade_apply_chains_version_1_to_3() {
    let root = make_version_1_descendant();
    let output: serde_json::Value =
        serde_json::from_str(&upgrade_apply_json(&root).unwrap()).unwrap();
    assert_eq!(output["ok"], true);
    assert_eq!(output["apply"], true);
    assert_eq!(output["fromVersion"], 1);
    assert_eq!(output["toVersion"], 3);
    assert_eq!(output["upgradePath"], serde_json::json!(["1->2", "2->3"]));
    assert!(root.join("core-rs/src/verify.rs").exists());
    assert!(root.join("core-rs/src/embedded_template.rs").exists());
    assert!(root.join("scripts/stage-embedded-template.mjs").exists());
    assert!(root.join("core-rs/src/upgrade_catalog.rs").exists());
    let config = load_config(&root).unwrap();
    assert_eq!(config.template.scaffold_version, Some(3));
    assert!(validate_repo(&root).unwrap().ok);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn upgrade_apply_is_no_op_for_current_descendant() {
    let root = make_fixture_root();
    let output: serde_json::Value =
        serde_json::from_str(&upgrade_apply_json(&root).unwrap()).unwrap();
    assert_eq!(output["ok"], true);
    assert_eq!(output["compatibility"], "current");
    assert_eq!(output["upgradePath"], serde_json::json!([]));
    assert_eq!(output["changedFiles"], serde_json::json!([]));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn upgrade_refuses_older_than_supported_descendants() {
    let root = make_previous_version_descendant();
    let mut config = load_config(&root).unwrap();
    config.template.scaffold_version = Some(0);
    write_config(&root, &config).unwrap();
    let error = upgrade_apply_json(&root).unwrap_err().to_string();
    assert!(error.contains("recreate is recommended"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn inspect_json_derives_scoped_runtime_package_names_from_configured_identity() {
    let source_root = make_source_checkout_root();
    let target = unique_temp_path("ossplate-inspect-scoped");
    if target.exists() {
        fs::remove_dir_all(&target).unwrap();
    }

    create_scaffold_from(
        &source_root,
        &target,
        &IdentityOverrides {
            name: Some("Scoped Tool".to_string()),
            description: None,
            repository: Some("https://github.com/acme/scoped-tool".to_string()),
            license: None,
            author_name: None,
            author_email: None,
            rust_crate: Some("scoped-tool-core".to_string()),
            npm_package: Some("@acme/scoped-tool".to_string()),
            python_package: Some("scoped-tool-py".to_string()),
            command: Some("scoped-tool".to_string()),
        },
    )
    .unwrap();

    let output: serde_json::Value =
        serde_json::from_str(&inspect_repo_json(&target).unwrap()).unwrap();
    assert_eq!(
        output["config"]["packages"]["npm_package"],
        "@acme/scoped-tool"
    );
    assert_eq!(
        output["derived"]["runtimePackages"][0]["folder"],
        "wrapper-js/platform-packages/ossplate-darwin-arm64"
    );
    assert_eq!(
        output["derived"]["runtimePackages"][0]["packageName"],
        "@acme/scoped-tool-darwin-arm64"
    );

    fs::remove_dir_all(&source_root).unwrap();
    fs::remove_dir_all(&target).unwrap();
}

#[test]
fn inspect_json_omits_source_checkout_when_not_present() {
    let root = make_fixture_root();
    fs::remove_file(root.join("source-checkout.json")).unwrap();
    let output: serde_json::Value =
        serde_json::from_str(&inspect_repo_json(&root).unwrap()).unwrap();
    assert!(output.get("sourceCheckout").is_none());
}

#[test]
fn create_json_returns_effective_identity() {
    let target = unique_temp_path("ossplate-create-json");
    if target.exists() {
        fs::remove_dir_all(&target).unwrap();
    }
    let output: serde_json::Value = serde_json::from_str(
        &create_scaffold_json(
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
        .unwrap(),
    )
    .unwrap();
    assert_eq!(output["ok"], true);
    assert_eq!(output["action"], "create");
    assert_eq!(output["created"], true);
    assert_eq!(output["config"]["packages"]["command"], "agentcode");
    fs::remove_dir_all(&target).unwrap();
}

#[test]
fn create_json_emits_only_json() {
    let target = unique_temp_path("ossplate-create-json-clean");
    if target.exists() {
        fs::remove_dir_all(&target).unwrap();
    }
    let rendered = create_scaffold_json(
        &target,
        &IdentityOverrides {
            name: Some("Json Tool".to_string()),
            description: None,
            repository: None,
            license: None,
            author_name: None,
            author_email: None,
            rust_crate: None,
            npm_package: None,
            python_package: None,
            command: Some("json-tool".to_string()),
        },
    )
    .unwrap();
    let output: serde_json::Value = serde_json::from_str(&rendered).unwrap();
    assert_eq!(output["ok"], true);
    fs::remove_dir_all(&target).unwrap();
}

#[test]
fn init_json_returns_effective_identity() {
    let source_root = make_source_checkout_root();
    let target = unique_temp_path("ossplate-init-json");
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
        "[package]\nname = \"bad-core\"\nversion = \"0.1.22\"\n",
    )
    .unwrap();

    let output: serde_json::Value = serde_json::from_str(
        &init_scaffold_json(
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
        .unwrap(),
    )
    .unwrap();
    assert_eq!(output["ok"], true);
    assert_eq!(output["action"], "init");
    assert_eq!(output["initialized"], true);
    assert_eq!(output["config"]["packages"]["command"], "agentcode");
    fs::remove_dir_all(&source_root).unwrap();
    fs::remove_dir_all(&target).unwrap();
}

#[test]
fn publish_plan_json_returns_helper_invocation() {
    let root = make_fixture_root();
    let output: serde_json::Value = serde_json::from_str(
        &render_publish_plan(&root, true, PublishRegistry::Pypi, true).unwrap(),
    )
    .unwrap();
    assert_eq!(output["ok"], true);
    assert_eq!(output["registry"], "Pypi");
    assert_eq!(output["dryRun"], true);
    assert_eq!(output["skipExisting"], true);
    assert_eq!(output["selectedRegistries"], serde_json::json!(["pypi"]));
    assert!(output["host"]["target"].is_string());
    assert!(output["helper"].as_str().unwrap().starts_with('/'));
    assert!(output["helper"]
        .as_str()
        .unwrap()
        .ends_with("scripts/publish-local.mjs"));
    assert!(output["argv"][0].as_str().unwrap().starts_with('/'));
    assert!(output["argv"].as_array().unwrap().len() >= 5);
    assert!(output["preflight"]["tools"].is_array());
    assert!(output["preflight"]["auth"].is_array());
    assert!(output["preflight"]["issues"].is_array());
    assert_eq!(output["preflight"]["tools"][2]["name"], "npm");
    assert_eq!(output["preflight"]["tools"][2]["required"], false);
    assert_eq!(output["preflight"]["tools"][2]["requiredForPublish"], false);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn publish_plan_json_marks_publish_only_requirements_separately() {
    let root = make_fixture_root();
    let output: serde_json::Value = serde_json::from_str(
        &render_publish_plan(&root, true, PublishRegistry::All, false).unwrap(),
    )
    .unwrap();
    let tools = output["preflight"]["tools"].as_array().unwrap();
    let npm = tools.iter().find(|tool| tool["name"] == "npm").unwrap();
    assert_eq!(npm["required"], false);
    assert_eq!(npm["requiredForPublish"], true);
    let auth = output["preflight"]["auth"].as_array().unwrap();
    let npm_auth = auth.iter().find(|entry| entry["kind"] == "npm").unwrap();
    assert_eq!(npm_auth["required"], false);
    assert_eq!(npm_auth["requiredForPublish"], true);
    fs::remove_dir_all(&root).unwrap();
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
fn create_uses_generated_placeholders_instead_of_template_maintainer_identity() {
    let source_root = make_source_checkout_root();
    let target = unique_temp_path("ossplate-generated-identity");
    if target.exists() {
        fs::remove_dir_all(&target).unwrap();
    }

    create_scaffold_from(
        &source_root,
        &target,
        &IdentityOverrides {
            name: Some("Ossblade".to_string()),
            description: None,
            repository: None,
            license: None,
            author_name: None,
            author_email: None,
            rust_crate: Some("ossblade".to_string()),
            npm_package: Some("ossblade".to_string()),
            python_package: Some("ossblade".to_string()),
            command: Some("ossblade".to_string()),
        },
    )
    .unwrap();

    let config = load_config(&target).unwrap();
    assert_eq!(
        config.project.description,
        generated_project_description(&config.packages.command)
    );
    assert_eq!(config.project.repository, GENERATED_REPOSITORY_PLACEHOLDER);
    assert_eq!(config.author.name, GENERATED_AUTHOR_NAME_PLACEHOLDER);
    assert_eq!(config.author.email, GENERATED_AUTHOR_EMAIL_PLACEHOLDER);
    assert!(!config.template.is_canonical);

    let validation = validate_repo(&target).unwrap();
    assert!(validation.ok);
    assert!(validation.warnings.len() >= 4);

    let readme = fs::read_to_string(target.join("README.md")).unwrap();
    assert!(readme.contains("cargo run --manifest-path core-rs/Cargo.toml -- validate --json"));
    assert!(!readme.contains("ossplate create"));
    assert!(!readme.contains("stefdevscore/ossplate"));

    let docs_index = fs::read_to_string(target.join("docs/README.md")).unwrap();
    assert!(docs_index.contains("generated `ossblade` project"));
    assert!(!docs_index.contains("Adoption Guide"));
    assert!(!target.join("docs/customizing-the-template.md").exists());
    assert!(!target.join("docs/live-e2e.md").exists());
    let scaffold_payload: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(target.join("scaffold-payload.json")).unwrap())
            .unwrap();
    let source_checkout: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(target.join("source-checkout.json")).unwrap())
            .unwrap();
    assert!(!scaffold_payload["requiredPaths"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry == "docs/customizing-the-template.md"));
    assert!(!source_checkout["requiredPaths"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry == "docs/live-e2e.md"));

    let release_guide = fs::read_to_string(target.join("docs/releases.md")).unwrap();
    assert!(release_guide.contains("crates.io publishes `ossblade`"));
    assert!(release_guide.contains("npm publishes `ossblade`"));
    assert!(!release_guide.contains("publish `ossplate`"));

    let wrapper_package: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(target.join("wrapper-js/package.json")).unwrap())
            .unwrap();
    assert_eq!(wrapper_package["name"], "ossblade");
    assert_eq!(
        wrapper_package["repository"]["url"],
        GENERATED_REPOSITORY_PLACEHOLDER
    );
    assert_eq!(
        wrapper_package["author"],
        "TODO: set author name <you@example.com>"
    );

    let package_lock: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(target.join("wrapper-js/package-lock.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(package_lock["name"], "ossblade");
    assert_eq!(package_lock["packages"][""]["name"], "ossblade");
    assert!(package_lock["packages"]
        .get("node_modules/ossblade-darwin-arm64")
        .is_some());
    assert!(package_lock["packages"]
        .get("node_modules/ossplate-darwin-arm64")
        .is_none());
    assert!(
        package_lock["packages"]["node_modules/ossblade-darwin-arm64"]
            .get("resolved")
            .is_none()
    );

    let embedded_config =
        fs::read_to_string(target.join("core-rs/embedded-template-root/ossplate.toml")).unwrap();
    assert!(embedded_config.contains("name = \"Ossblade\""));
    assert!(embedded_config.contains("is_canonical = false"));
    assert!(embedded_config
        .contains("repository = \"https://example.com/replace-with-your-repository\""));
    assert!(!embedded_config.contains("stefdevscore/ossplate"));
    assert!(!target
        .join("core-rs/embedded-template-root/docs/customizing-the-template.md")
        .exists());
    assert!(!target
        .join("core-rs/embedded-template-root/docs/live-e2e.md")
        .exists());
    assert!(!target
        .join("core-rs/generated-embedded-template-root")
        .exists());

    fs::remove_dir_all(&source_root).unwrap();
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
    let config = load_config(&target).unwrap();
    assert!(!config.template.is_canonical);
    assert!(target.join("wrapper-js/package.json").exists());
    assert!(target.join("wrapper-py/pyproject.toml").exists());
    assert!(validate_repo(&target).unwrap().ok);

    fs::remove_dir_all(&target).unwrap();
}

#[test]
fn init_preserves_resolved_runtime_entries_when_npm_identity_is_unchanged() {
    let source_root = make_source_checkout_root();
    let target = unique_temp_path("ossplate-init-lockfile-preserve");
    if target.exists() {
        fs::remove_dir_all(&target).unwrap();
    }

    create_scaffold_from(&source_root, &target, &IdentityOverrides::default()).unwrap();

    let lock_path = target.join("wrapper-js/package-lock.json");
    let mut lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&lock_path).unwrap()).unwrap();
    let runtime_entry = lock["packages"]
        .get_mut("node_modules/ossplate-darwin-arm64")
        .unwrap()
        .as_object_mut()
        .unwrap();
    runtime_entry.insert(
        "resolved".into(),
        serde_json::Value::String("https://registry.npmjs.org/ossplate-darwin-arm64".to_string()),
    );
    runtime_entry.insert(
        "integrity".into(),
        serde_json::Value::String("sha512-test".to_string()),
    );
    let mut rendered = serde_json::to_string_pretty(&lock).unwrap();
    rendered.push('\n');
    fs::write(&lock_path, rendered).unwrap();

    init_scaffold_from(&source_root, &target, &IdentityOverrides::default()).unwrap();

    let reloaded: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&lock_path).unwrap()).unwrap();
    let runtime_entry = reloaded["packages"]
        .get("node_modules/ossplate-darwin-arm64")
        .unwrap();
    assert_eq!(
        runtime_entry
            .get("resolved")
            .and_then(serde_json::Value::as_str),
        Some("https://registry.npmjs.org/ossplate-darwin-arm64")
    );
    assert_eq!(
        runtime_entry
            .get("integrity")
            .and_then(serde_json::Value::as_str),
        Some("sha512-test")
    );

    fs::remove_dir_all(&source_root).unwrap();
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
    assert_release_check_scaffold_assets(&target);

    let cargo_toml = fs::read_to_string(target.join("core-rs/Cargo.toml")).unwrap();
    let cargo_lock = fs::read_to_string(target.join("core-rs/Cargo.lock")).unwrap();
    assert!(cargo_toml.contains("name = \"agentcode\""));
    assert!(cargo_toml.contains("default-run = \"agentcode\""));
    assert!(cargo_toml.contains("[[bin]]"));
    assert!(cargo_lock.contains("name = \"agentcode\""));

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
    assert!(pyproject.contains("keywords = [\"cli\", \"bootstrap\", \"distribution\", \"packaging\", \"multi-registry\", \"rust\", \"python\", \"npm\"]"));
    assert!(target.join("wrapper-py/src/agentcode/cli.py").exists());
    assert!(!target.join("wrapper-py/src/ossplate/cli.py").exists());

    let generated_readme = fs::read_to_string(target.join("README.md")).unwrap();
    let generated_docs_index = fs::read_to_string(target.join("docs/README.md")).unwrap();
    let generated_agent_ops = fs::read_to_string(target.join("docs/agent-operations.md")).unwrap();
    assert!(generated_readme.contains("docs/agent-operations.md"));
    assert!(!generated_readme.contains("docs/agents.md"));
    assert!(generated_docs_index.contains("./agent-operations.md"));
    assert!(!generated_docs_index.contains("./agents.md"));
    assert!(!generated_agent_ops.contains("ossplate create"));
    assert!(!generated_agent_ops.contains("ossplate init"));

    fs::remove_dir_all(&target).unwrap();
}

#[test]
fn metadata_sync_uses_config_owned_discoverability_fields() {
    let root = make_fixture_root();
    fs::write(
        root.join("core-rs/Cargo.toml"),
        fs::read_to_string(root.join("core-rs/Cargo.toml"))
            .unwrap()
            .replace("keywords = [\"cli\", \"bootstrap\", \"distribution\", \"packaging\", \"multi-registry\"]", "keywords = [\"wrong\"]")
            .replace("categories = [\"command-line-utilities\", \"development-tools\"]", "categories = [\"wrong\"]"),
    )
    .unwrap();
    fs::write(
        root.join("wrapper-js/package.json"),
        fs::read_to_string(root.join("wrapper-js/package.json"))
            .unwrap()
            .replace("\"keywords\": [\n    \"bootstrap\",\n    \"distribution\",\n    \"packaging\",\n    \"multi-registry\",\n    \"rust\",\n    \"python\",\n    \"npm\",\n    \"cli\"\n  ],", "\"keywords\": [\"wrong\"],"),
    )
    .unwrap();
    fs::write(
        root.join("wrapper-py/pyproject.toml"),
        fs::read_to_string(root.join("wrapper-py/pyproject.toml"))
            .unwrap()
            .replace("keywords = [\"cli\", \"bootstrap\", \"distribution\", \"packaging\", \"multi-registry\", \"rust\", \"python\", \"npm\"]", "keywords = [\"wrong\"]")
            .replace(
                "classifiers = [\n  \"Development Status :: 4 - Beta\",\n  \"Environment :: Console\",\n  \"Intended Audience :: Developers\",\n  \"Programming Language :: Python :: 3\",\n  \"Topic :: Software Development :: Build Tools\",\n  \"Topic :: Software Development :: Code Generators\",\n  \"Topic :: Utilities\",\n  \"License :: Public Domain\",\n  \"Operating System :: OS Independent\"\n]",
                "classifiers = [\"Wrong :: Classifier\"]",
            ),
    )
    .unwrap();

    sync_repo(&root, false).unwrap();
    assert!(validate_repo(&root).unwrap().ok);
    let cargo = fs::read_to_string(root.join("core-rs/Cargo.toml")).unwrap();
    let js = fs::read_to_string(root.join("wrapper-js/package.json")).unwrap();
    let py = fs::read_to_string(root.join("wrapper-py/pyproject.toml")).unwrap();
    assert!(cargo.contains(
        "keywords = [\"cli\", \"bootstrap\", \"distribution\", \"packaging\", \"multi-registry\"]"
    ));
    assert!(cargo.contains("categories = [\"command-line-utilities\", \"development-tools\"]"));
    assert!(js.contains("\"multi-registry\""));
    assert!(py.contains("keywords = [\"cli\", \"bootstrap\", \"distribution\", \"packaging\", \"multi-registry\", \"rust\", \"python\", \"npm\"]"));
    assert!(py.contains("\"Topic :: Utilities\""));
}

#[test]
fn create_skips_local_generated_artifacts_from_template_source() {
    let source_root = make_source_checkout_root();
    fs::create_dir_all(source_root.join(".dist-assets/runtime/linux-x64")).unwrap();
    fs::create_dir_all(source_root.join(".live-e2e")).unwrap();
    fs::create_dir_all(source_root.join("wrapper-py/.tmp-inspect")).unwrap();
    fs::create_dir_all(source_root.join("core-rs/target/debug")).unwrap();
    fs::write(
        source_root.join(".dist-assets/runtime/linux-x64/ossplate"),
        "generated-binary",
    )
    .unwrap();
    fs::write(source_root.join(".live-e2e/generated.log"), "generated-log").unwrap();
    fs::write(
        source_root.join("wrapper-py/.tmp-inspect/generated.txt"),
        "generated-temp",
    )
    .unwrap();
    fs::write(
        source_root.join("core-rs/target/debug/ossplate"),
        "generated-target",
    )
    .unwrap();

    let target = unique_temp_path("ossplate-create-clean-source-copy");
    if target.exists() {
        fs::remove_dir_all(&target).unwrap();
    }

    create_scaffold_from(&source_root, &target, &IdentityOverrides::default()).unwrap();

    assert!(!target.join(".dist-assets").exists());
    assert!(!target.join(".live-e2e").exists());
    assert!(!target.join("wrapper-py/.tmp-inspect").exists());
    assert!(!target.join("core-rs/target").exists());

    fs::remove_dir_all(&source_root).unwrap();
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
fn sync_restores_canonical_root_readme_content() {
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
    assert!(synced.contains("## Learn More"));
    assert!(synced.contains(&config.project.description));
}

#[test]
fn template_detection_does_not_depend_on_exact_maintainer_identity() {
    let root = make_fixture_root();
    let mut config = load_config(&root).unwrap();
    config.project.repository = "https://github.com/acme/ossplate".to_string();
    config.author.name = "Acme".to_string();
    config.author.email = "oss@acme.dev".to_string();

    assert!(is_template_project(&config));
}

#[test]
fn template_detection_uses_explicit_canonical_marker() {
    let root = make_fixture_root();
    let mut config = load_config(&root).unwrap();
    config.project.name = "Rebranded Template".to_string();
    config.packages.command = "rebrand".to_string();
    config.packages.rust_crate = "rebrand".to_string();
    config.packages.npm_package = "rebrand".to_string();
    config.packages.python_package = "rebrand".to_string();
    config.template.is_canonical = true;

    assert!(is_template_project(&config));

    config.template.is_canonical = false;
    assert!(!is_template_project(&config));
}

#[test]
fn template_detection_trusts_explicit_canonical_marker_even_with_placeholders() {
    let root = make_fixture_root();
    let mut config = load_config(&root).unwrap();
    config.template.is_canonical = true;
    config.project.repository = GENERATED_REPOSITORY_PLACEHOLDER.to_string();
    config.author.name = GENERATED_AUTHOR_NAME_PLACEHOLDER.to_string();
    config.author.email = GENERATED_AUTHOR_EMAIL_PLACEHOLDER.to_string();

    assert!(is_template_project(&config));
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

#[test]
fn embedded_template_root_materializes_required_scaffold_files() {
    let root = materialize_embedded_template_root().unwrap();
    assert!(embedded_template_contains(&root, "ossplate.toml"));
    assert!(embedded_template_contains(&root, "scaffold-payload.json"));
    assert!(embedded_template_contains(&root, "source-checkout.json"));
    assert!(embedded_template_contains(&root, "core-rs/Cargo.toml"));
    assert!(embedded_template_contains(
        &root,
        "core-rs/embedded-template-root/ossplate.toml"
    ));
    assert!(embedded_template_contains(&root, "core-rs/src/main.rs"));
    assert!(embedded_template_contains(
        &root,
        "core-rs/embedded-template-root/core-rs/Cargo.toml"
    ));
    assert!(embedded_template_contains(
        &root,
        "core-rs/embedded-template-root/core-rs/src/main.rs"
    ));
    assert!(embedded_template_contains(&root, "wrapper-js/package.json"));
    assert!(embedded_template_contains(
        &root,
        "wrapper-py/pyproject.toml"
    ));
    assert!(!embedded_template_contains(
        &root,
        "docs/customizing-the-template.md"
    ));
    ensure_scaffold_source_root(&root).unwrap();
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn create_scaffold_from_embedded_template_root_preserves_create_contract() {
    let source_root = materialize_embedded_template_root().unwrap();
    let target = unique_temp_path("ossplate-embedded-create");
    if target.exists() {
        fs::remove_dir_all(&target).unwrap();
    }

    create_scaffold_from(&source_root, &target, &IdentityOverrides::default()).unwrap();
    assert!(target.join("ossplate.toml").is_file());
    assert!(target.join("README.md").is_file());
    assert!(target.join("wrapper-js/package.json").is_file());
    assert!(target.join("wrapper-py/pyproject.toml").is_file());
    assert_eq!(
        inspect_compatibility(&target).unwrap().compatibility,
        Compatibility::Current
    );
    ensure_scaffold_source_root(&target.join("core-rs").join("embedded-template-root")).unwrap();
    assert_release_check_scaffold_assets(&target);

    fs::remove_dir_all(&source_root).unwrap();
    fs::remove_dir_all(&target).unwrap();
}

#[test]
fn sync_repo_keeps_cargo_template_manifest_aligned() {
    let root = make_source_checkout_root();
    fs::write(
        root.join("core-rs/Cargo.template.toml"),
        fs::read_to_string(repo_root().join("core-rs/Cargo.template.toml")).unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("core-rs/Cargo.template.toml"),
        fs::read_to_string(root.join("core-rs/Cargo.template.toml"))
            .unwrap()
            .replace("version = \"0.5.3\"", "version = \"9.9.9\"")
            .replace("name = \"ossplate\"", "name = \"wrong-template\""),
    )
    .unwrap();

    sync_repo(&root, false).unwrap();

    let cargo_template = fs::read_to_string(root.join("core-rs/Cargo.template.toml")).unwrap();
    let expected = fs::read_to_string(repo_root().join("core-rs/Cargo.template.toml")).unwrap();
    let actual: toml::Value = toml::from_str(&cargo_template).unwrap();
    let expected: toml::Value = toml::from_str(&expected).unwrap();
    assert_eq!(actual, expected);

    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn create_from_generated_embedded_template_root_preserves_generated_identity_baseline() {
    let source_root = make_source_checkout_root();
    let generated_root = unique_temp_path("ossplate-generated-embedded-source");
    if generated_root.exists() {
        fs::remove_dir_all(&generated_root).unwrap();
    }

    create_scaffold_from(
        &source_root,
        &generated_root,
        &IdentityOverrides {
            name: Some("Dogfood Control Plane".to_string()),
            description: Some(
                "Ship the dogfood-control CLI across Cargo, npm, and PyPI.".to_string(),
            ),
            repository: Some("https://github.com/acme/dogfood-control".to_string()),
            license: Some("Apache-2.0".to_string()),
            author_name: Some("Acme OSS".to_string()),
            author_email: Some("oss@acme.dev".to_string()),
            rust_crate: Some("dogfood-control".to_string()),
            npm_package: Some("@acme/dogfood-control".to_string()),
            python_package: Some("dogfood_control".to_string()),
            command: Some("dogfood-control".to_string()),
        },
    )
    .unwrap();

    let rebootstrap_root = unique_temp_path("ossplate-generated-embedded-rebootstrap");
    if rebootstrap_root.exists() {
        fs::remove_dir_all(&rebootstrap_root).unwrap();
    }

    create_scaffold_from(
        &generated_root.join("core-rs/embedded-template-root"),
        &rebootstrap_root,
        &IdentityOverrides::default(),
    )
    .unwrap();

    let config = load_config(&rebootstrap_root).unwrap();
    assert_eq!(config.project.name, "Dogfood Control Plane");
    assert_eq!(
        config.project.description,
        "Ship the dogfood-control CLI across Cargo, npm, and PyPI."
    );
    assert_eq!(
        config.project.repository,
        "https://github.com/acme/dogfood-control"
    );
    assert_eq!(config.author.name, "Acme OSS");
    assert_eq!(config.author.email, "oss@acme.dev");
    assert_eq!(config.packages.rust_crate, "dogfood-control");
    assert_eq!(config.packages.npm_package, "@acme/dogfood-control");
    assert_eq!(config.packages.python_package, "dogfood_control");
    assert_eq!(config.packages.command, "dogfood-control");
    assert!(!config.template.is_canonical);

    fs::remove_dir_all(&source_root).unwrap();
    fs::remove_dir_all(&generated_root).unwrap();
    fs::remove_dir_all(&rebootstrap_root).unwrap();
}

fn assert_release_check_scaffold_assets(root: &Path) {
    let status = Command::new("node")
        .arg(root.join("scripts/release-check.mjs"))
        .arg("scaffold-assets")
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success(), "expected scaffold asset check to pass");
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

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn make_source_checkout_root() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("ossplate-source-fixture-{unique}"));
    let repo_root = repo_root();
    copy_required_paths_from_manifest(&repo_root, &root);
    root
}

fn make_previous_version_descendant() -> PathBuf {
    let source_root = repo_root();
    let target = unique_temp_path("ossplate-previous-descendant");
    create_scaffold_from(&source_root, &target, &IdentityOverrides::default()).unwrap();

    let mut config = load_config(&target).unwrap();
    config.template.scaffold_version = Some(2);
    write_config(&target, &config).unwrap();

    remove_paths_for_version(&target, &["core-rs/src/upgrade_catalog.rs"]);

    target
}

fn make_version_1_descendant() -> PathBuf {
    let target = make_previous_version_descendant();

    let mut config = load_config(&target).unwrap();
    config.template.scaffold_version = Some(1);
    write_config(&target, &config).unwrap();

    let removed_paths = [
        "core-rs/build.rs",
        "core-rs/src/embedded_template.rs",
        "core-rs/src/upgrade.rs",
        "core-rs/src/verify.rs",
        "scripts/stage-embedded-template.mjs",
        "scripts/package-js.mjs",
    ];
    remove_paths_for_version(&target, &removed_paths);

    target
}

fn remove_paths_for_version(root: &Path, removed_paths: &[&str]) {
    for relative_path in removed_paths {
        let target = root.join(relative_path);
        if target.exists() {
            fs::remove_file(target).unwrap();
        }
    }
    for manifest_path in [
        root.join("scaffold-payload.json"),
        root.join("source-checkout.json"),
        root.join("core-rs/source-checkout.json"),
    ] {
        remove_required_paths_from_json(&manifest_path, removed_paths);
    }
}

fn remove_required_path_from_json(path: &Path, required_path: &str) {
    remove_required_paths_from_json(path, &[required_path]);
}

fn remove_required_paths_from_json(path: &Path, required_paths_to_remove: &[&str]) {
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    let required_paths = value["requiredPaths"].as_array_mut().unwrap();
    required_paths.retain(|entry| {
        entry
            .as_str()
            .is_none_or(|path| !required_paths_to_remove.contains(&path))
    });
    fs::write(path, serde_json::to_string_pretty(&value).unwrap() + "\n").unwrap();
}

fn copy_required_paths_from_manifest(source_root: &Path, target_root: &Path) {
    fs::create_dir_all(target_root).unwrap();
    fs::copy(
        source_root.join("scaffold-payload.json"),
        target_root.join("scaffold-payload.json"),
    )
    .unwrap();
    fs::copy(
        source_root.join("source-checkout.json"),
        target_root.join("source-checkout.json"),
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
