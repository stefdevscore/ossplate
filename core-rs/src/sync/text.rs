use crate::config::{is_template_project, ToolConfig};
use crate::sync::ValidationIssue;
use anyhow::{anyhow, bail, Result};
use std::collections::BTreeMap;

pub(crate) const README_IDENTITY_START: &str = "<!-- ossplate:readme-identity:start -->";
pub(crate) const README_IDENTITY_END: &str = "<!-- ossplate:readme-identity:end -->";
pub(crate) const WORKFLOW_NAME_START: &str = "# ossplate:workflow-name:start";
pub(crate) const WORKFLOW_NAME_END: &str = "# ossplate:workflow-name:end";

pub(crate) fn validate_js_readme(
    config: &ToolConfig,
    content: &str,
) -> Result<Vec<ValidationIssue>> {
    validate_wrapper_readme("wrapper-js/README.md", "JavaScript", config, content)
}

pub(crate) fn sync_js_readme(config: &ToolConfig, content: &str) -> Result<String> {
    Ok(render_wrapper_readme_with_newlines(
        "JavaScript",
        config,
        detect_newline_style(content),
    ))
}

pub(crate) fn validate_py_readme(
    config: &ToolConfig,
    content: &str,
) -> Result<Vec<ValidationIssue>> {
    validate_wrapper_readme("wrapper-py/README.md", "Python", config, content)
}

pub(crate) fn sync_py_readme(config: &ToolConfig, content: &str) -> Result<String> {
    Ok(render_wrapper_readme_with_newlines(
        "Python",
        config,
        detect_newline_style(content),
    ))
}

pub(crate) fn validate_root_readme(
    config: &ToolConfig,
    content: &str,
) -> Result<Vec<ValidationIssue>> {
    let expected = render_root_readme_with_newlines(config, detect_newline_style(content));
    if normalize_newlines(content) == normalize_newlines(&expected) {
        Ok(Vec::new())
    } else {
        Ok(vec![issue(
            "README.md",
            "readme.identity",
            "owned metadata differs from the canonical project identity",
            Some(expected),
            Some(content.to_string()),
        )])
    }
}

pub(crate) fn sync_root_readme(config: &ToolConfig, content: &str) -> Result<String> {
    Ok(render_root_readme_with_newlines(
        config,
        detect_newline_style(content),
    ))
}

pub(crate) fn validate_docs_index(
    config: &ToolConfig,
    content: &str,
) -> Result<Vec<ValidationIssue>> {
    let expected = render_docs_index_with_newlines(config, detect_newline_style(content));
    if normalize_newlines(content) == normalize_newlines(&expected) {
        Ok(Vec::new())
    } else {
        Ok(vec![issue(
            "docs/README.md",
            "docs.index",
            "owned metadata differs from the canonical project identity",
            Some(expected),
            Some(content.to_string()),
        )])
    }
}

pub(crate) fn sync_docs_index(config: &ToolConfig, content: &str) -> Result<String> {
    Ok(render_docs_index_with_newlines(
        config,
        detect_newline_style(content),
    ))
}

pub(crate) fn validate_releases_doc(
    config: &ToolConfig,
    content: &str,
) -> Result<Vec<ValidationIssue>> {
    let expected = render_releases_doc_with_newlines(config, detect_newline_style(content));
    if normalize_newlines(content) == normalize_newlines(&expected) {
        Ok(Vec::new())
    } else {
        Ok(vec![issue(
            "docs/releases.md",
            "docs.releases",
            "owned metadata differs from the canonical project identity",
            Some(expected),
            Some(content.to_string()),
        )])
    }
}

pub(crate) fn sync_releases_doc(config: &ToolConfig, content: &str) -> Result<String> {
    Ok(render_releases_doc_with_newlines(
        config,
        detect_newline_style(content),
    ))
}

pub(crate) fn validate_ci_workflow(
    config: &ToolConfig,
    content: &str,
) -> Result<Vec<ValidationIssue>> {
    validate_workflow_name(
        ".github/workflows/ci.yml",
        &format!("{} CI", config.project.name),
        content,
    )
}

pub(crate) fn sync_ci_workflow(config: &ToolConfig, content: &str) -> Result<String> {
    sync_workflow_name(content, &format!("{} CI", config.project.name))
}

pub(crate) fn validate_publish_workflow(
    config: &ToolConfig,
    content: &str,
) -> Result<Vec<ValidationIssue>> {
    validate_workflow_name(
        ".github/workflows/publish.yml",
        &format!("{} publishing", config.project.name),
        content,
    )
}

pub(crate) fn sync_publish_workflow(config: &ToolConfig, content: &str) -> Result<String> {
    sync_workflow_name(content, &format!("{} publishing", config.project.name))
}

pub(crate) fn validate_publish_npm_workflow(
    config: &ToolConfig,
    content: &str,
) -> Result<Vec<ValidationIssue>> {
    validate_workflow_name(
        ".github/workflows/publish-npm.yml",
        &format!("{} publish-npm", config.project.name),
        content,
    )
}

pub(crate) fn sync_publish_npm_workflow(config: &ToolConfig, content: &str) -> Result<String> {
    sync_workflow_name(content, &format!("{} publish-npm", config.project.name))
}

pub(crate) fn validate_live_e2e_workflow(
    config: &ToolConfig,
    content: &str,
) -> Result<Vec<ValidationIssue>> {
    validate_workflow_name(
        ".github/workflows/live-e2e-published.yml",
        &format!("{} live-e2e", config.project.name),
        content,
    )
}

pub(crate) fn sync_live_e2e_workflow(config: &ToolConfig, content: &str) -> Result<String> {
    sync_workflow_name(content, &format!("{} live-e2e", config.project.name))
}

fn validate_wrapper_readme(
    path: &str,
    language: &str,
    config: &ToolConfig,
    content: &str,
) -> Result<Vec<ValidationIssue>> {
    let expected = render_wrapper_readme(language, config);
    if normalize_newlines(content) == normalize_newlines(&expected) {
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

pub(crate) fn render_wrapper_readme(language: &str, config: &ToolConfig) -> String {
    render_wrapper_readme_with_newlines(language, config, "\n")
}

fn render_wrapper_readme_with_newlines(
    language: &str,
    config: &ToolConfig,
    newline: &str,
) -> String {
    if !is_template_project(config) {
        let docs_url = relative_docs_link("README.md");
        let agent_ops_url = relative_docs_link("agent-operations.md");
        let testing_url = relative_docs_link("testing.md");
        let releases_url = relative_docs_link("releases.md");
        return [
            format!("# {}", config.project.name),
            String::new(),
            format!(
                "This package installs the {} delivery adapter for the `{}` CLI.",
                language, config.packages.command
            ),
            String::new(),
            "The generated project keeps one Rust-core CLI and ships the same command through Cargo, npm, and PyPI. This wrapper forwards to the bundled native binary for the current platform.".to_string(),
            String::new(),
            "Common commands:".to_string(),
            String::new(),
            "```bash".to_string(),
            format!("{} version", config.packages.command),
            format!("{} validate --json", config.packages.command),
            format!("{} inspect --json", config.packages.command),
            format!("{} sync --check --json", config.packages.command),
            format!("{} verify --json", config.packages.command),
            "```".to_string(),
            String::new(),
            "If you are working from a source checkout instead of an installed package, use the same subcommands through:".to_string(),
            String::new(),
            "```bash".to_string(),
            "cargo run --manifest-path core-rs/Cargo.toml -- <subcommand> ...".to_string(),
            "```".to_string(),
            String::new(),
            "Repo-local documentation:".to_string(),
            String::new(),
            "- [Project overview](../README.md)".to_string(),
            format!("- [Documentation index]({docs_url})"),
            format!("- [Agent Operations]({agent_ops_url})"),
            format!("- [Testing guide]({testing_url})"),
            format!("- [Release guide]({releases_url})"),
            String::new(),
        ]
        .join(newline);
    }

    let image_url = github_raw_url(
        &config.project.repository,
        "main",
        "assets/illustrations/chestplate.svg",
    )
    .expect("wrapper README rendering requires a GitHub repository URL");
    let docs_url = github_blob_url(&config.project.repository, "main", "docs/README.md")
        .expect("wrapper README rendering requires a GitHub repository URL");
    let testing_url = github_blob_url(&config.project.repository, "main", "docs/testing.md")
        .expect("wrapper README rendering requires a GitHub repository URL");
    let architecture_url =
        github_blob_url(&config.project.repository, "main", "docs/architecture.md")
            .expect("wrapper README rendering requires a GitHub repository URL");
    let agent_ops_url = github_blob_url(
        &config.project.repository,
        "main",
        "docs/agent-operations.md",
    )
    .expect("wrapper README rendering requires a GitHub repository URL");
    [
        format!("# {}", config.project.name),
        String::new(),
        "<p align=\"center\">".to_string(),
        format!(
            "  <img src=\"{}\" alt=\"{} armor\" width=\"320\">",
            image_url, config.project.name
        ),
        "</p>".to_string(),
        String::new(),
        format!(
            "`{}` helps you start and maintain a project that ships the same CLI through Rust, npm, and PyPI.",
            config.packages.command
        ),
        String::new(),
        "Use it to:".to_string(),
        String::new(),
        "- create a new scaffolded project".to_string(),
        "- initialize an existing directory".to_string(),
        "- validate project identity and metadata".to_string(),
        "- keep owned files in sync".to_string(),
        "- inspect the effective repo contract".to_string(),
        "- run the full repo gate in structured JSON".to_string(),
        String::new(),
        format!(
            "This package is the installed {} delivery adapter for the same `{}` CLI described in the main docs. It forwards to the bundled native binary for your current platform and exposes the same subcommands as the Rust core.",
            language,
            config.packages.command
        ),
        String::new(),
        "Common commands:".to_string(),
        String::new(),
        "```bash".to_string(),
        format!("{} version", config.packages.command),
        format!("{} create <target>", config.packages.command),
        format!("{} init --path <dir>", config.packages.command),
        format!("{} validate --json", config.packages.command),
        format!("{} inspect --json", config.packages.command),
        format!("{} sync --check --json", config.packages.command),
        format!("{} verify --json", config.packages.command),
        "```".to_string(),
        String::new(),
        "Typical workflow:".to_string(),
        String::new(),
        "```bash".to_string(),
        format!("{} create ../my-new-project \\", config.packages.command),
        "  --name \"My Project\" \\".to_string(),
        "  --repository \"https://github.com/acme/my-project\" \\".to_string(),
        "  --author-name \"Acme OSS\" \\".to_string(),
        "  --author-email \"oss@acme.dev\" \\".to_string(),
        "  --rust-crate \"my-project-core\" \\".to_string(),
        "  --npm-package \"@acme/my-project\" \\".to_string(),
        "  --python-package \"my-project-py\" \\".to_string(),
        "  --command \"my-project\"".to_string(),
        String::new(),
        format!("{} validate --path ../my-new-project --json", config.packages.command),
        format!("{} inspect --path ../my-new-project --json", config.packages.command),
        format!(
            "{} sync --path ../my-new-project --check --json",
            config.packages.command
        ),
        "```".to_string(),
        String::new(),
        format!(
            "If you are working from a source checkout instead of an installed {} package, use the same subcommands through:",
            if language == "JavaScript" { "npm" } else { "Python" }
        ),
        String::new(),
        "```bash".to_string(),
        "cargo run --manifest-path core-rs/Cargo.toml -- <subcommand> ...".to_string(),
        "```".to_string(),
        String::new(),
        "Learn more:".to_string(),
        String::new(),
        format!("- [Main documentation]({docs_url})"),
        format!("- [Agent Operations]({agent_ops_url})"),
        format!("- [Testing guide]({testing_url})"),
        format!("- [Architecture]({architecture_url})"),
        String::new(),
    ]
    .join(newline)
}

fn render_root_readme_with_newlines(config: &ToolConfig, newline: &str) -> String {
    if is_template_project(config) {
        return render_template_root_readme_with_newlines(config, newline);
    }

    [
        README_IDENTITY_START.to_string(),
        format!("# {}", config.project.name),
        String::new(),
        config.project.description.clone(),
        README_IDENTITY_END.to_string(),
        String::new(),
        format!(
            "`{}` ships one CLI through Cargo, npm, and PyPI from a single Rust core.",
            config.packages.command
        ),
        String::new(),
        "This generated repository keeps project identity, packaging metadata, and release operations aligned across the three distribution surfaces.".to_string(),
        String::new(),
        "## Local Commands".to_string(),
        String::new(),
        "Use the source checkout directly:".to_string(),
        String::new(),
        "```bash".to_string(),
        "cargo run --manifest-path core-rs/Cargo.toml -- version".to_string(),
        "cargo run --manifest-path core-rs/Cargo.toml -- validate --json".to_string(),
        "cargo run --manifest-path core-rs/Cargo.toml -- inspect --json".to_string(),
        "cargo run --manifest-path core-rs/Cargo.toml -- sync --check --json".to_string(),
        "cargo run --manifest-path core-rs/Cargo.toml -- verify --json".to_string(),
        "```".to_string(),
        String::new(),
        "If you install the published wrapper package for your ecosystem, use the same command through the installed CLI name:".to_string(),
        String::new(),
        "```bash".to_string(),
        format!("{} version", config.packages.command),
        format!("{} validate --json", config.packages.command),
        format!("{} inspect --json", config.packages.command),
        format!("{} sync --check --json", config.packages.command),
        format!("{} verify --json", config.packages.command),
        "```".to_string(),
        String::new(),
        "## Project Packages".to_string(),
        String::new(),
        format!("- Cargo: `{}`", config.packages.rust_crate),
        format!("- npm: `{}`", config.packages.npm_package),
        format!("- PyPI: `{}`", config.packages.python_package),
        format!("- CLI command: `{}`", config.packages.command),
        String::new(),
        "## Documentation".to_string(),
        String::new(),
        "- [Documentation index](./docs/README.md)".to_string(),
        "- [Agent Operations](./docs/agent-operations.md)".to_string(),
        "- [Testing Guide](./docs/testing.md)".to_string(),
        "- [Release Guide](./docs/releases.md)".to_string(),
        "- [Architecture](./docs/architecture.md)".to_string(),
        String::new(),
        "## License".to_string(),
        String::new(),
        format!("Licensed under the [{}](LICENSE).", config.project.license),
        String::new(),
    ]
    .join(newline)
}

fn render_template_root_readme_with_newlines(config: &ToolConfig, newline: &str) -> String {
    let image_url = github_raw_url(
        &config.project.repository,
        "main",
        "assets/illustrations/chestplate.svg",
    )
    .expect("template README rendering requires a GitHub repository URL");
    let docs_url = github_blob_url(&config.project.repository, "main", "docs/README.md")
        .expect("template README rendering requires a GitHub repository URL");
    let agent_ops_url = github_blob_url(
        &config.project.repository,
        "main",
        "docs/agent-operations.md",
    )
    .expect("template README rendering requires a GitHub repository URL");
    let adoption_url = github_blob_url(
        &config.project.repository,
        "main",
        "docs/customizing-the-template.md",
    )
    .expect("template README rendering requires a GitHub repository URL");
    let testing_url = github_blob_url(&config.project.repository, "main", "docs/testing.md")
        .expect("template README rendering requires a GitHub repository URL");
    let releases_url = github_blob_url(&config.project.repository, "main", "docs/releases.md")
        .expect("template README rendering requires a GitHub repository URL");
    let architecture_url =
        github_blob_url(&config.project.repository, "main", "docs/architecture.md")
            .expect("template README rendering requires a GitHub repository URL");

    [
        README_IDENTITY_START.to_string(),
        format!("# {}", config.project.name),
        String::new(),
        config.project.description.clone(),
        README_IDENTITY_END.to_string(),
        String::new(),
        "<p align=\"center\">".to_string(),
        format!(
            "  <img src=\"{}\" alt=\"{} armor\" width=\"360\">",
            image_url, config.project.name
        ),
        "</p>".to_string(),
        String::new(),
        format!(
            "`{}` helps maintainers and agents start and keep a single CLI aligned across Rust, npm, and PyPI.",
            config.packages.command
        ),
        String::new(),
        "It gives you a working baseline with:".to_string(),
        String::new(),
        "- one real core CLI".to_string(),
        "- thin JavaScript and Python wrappers".to_string(),
        "- release-ready workflows for Cargo, npm, and PyPI".to_string(),
        "- a scaffold you can create, adopt, and keep in sync".to_string(),
        "- machine-checkable validation, planning, repair, inspection, and verification commands for agent loops".to_string(),
        String::new(),
        "## Installed Usage".to_string(),
        String::new(),
        "```bash".to_string(),
        format!("{} version", config.packages.command),
        format!("{} create ../my-new-project \\", config.packages.command),
        "  --name \"My Project\" \\".to_string(),
        "  --repository \"https://github.com/acme/my-project\" \\".to_string(),
        "  --author-name \"Acme\" \\".to_string(),
        "  --author-email \"oss@acme.dev\" \\".to_string(),
        "  --rust-crate \"my-project\" \\".to_string(),
        "  --npm-package \"@acme/my-project\" \\".to_string(),
        "  --python-package \"my-project-py\" \\".to_string(),
        "  --command \"my-project\"".to_string(),
        "```".to_string(),
        String::new(),
        "## Source Checkout Usage".to_string(),
        String::new(),
        "- `cargo run --manifest-path core-rs/Cargo.toml -- create <target>`".to_string(),
        "- `cargo run --manifest-path core-rs/Cargo.toml -- validate --json`".to_string(),
        "- `cargo run --manifest-path core-rs/Cargo.toml -- inspect --json`".to_string(),
        "- `cargo run --manifest-path core-rs/Cargo.toml -- sync --check --json`".to_string(),
        "- `cargo run --manifest-path core-rs/Cargo.toml -- publish --plan --json`".to_string(),
        "- `cargo run --manifest-path core-rs/Cargo.toml -- verify --json`".to_string(),
        String::new(),
        "## Learn More".to_string(),
        String::new(),
        format!("- [Documentation]({docs_url})"),
        format!("- [Agent Operations]({agent_ops_url})"),
        format!("- [Adoption Guide]({adoption_url})"),
        format!("- [Testing Guide]({testing_url})"),
        format!("- [Release Guide]({releases_url})"),
        format!("- [Architecture]({architecture_url})"),
        String::new(),
        "## License".to_string(),
        String::new(),
        format!("Licensed under the [{}](LICENSE).", config.project.license),
        String::new(),
    ]
    .join(newline)
}

fn render_docs_index_with_newlines(config: &ToolConfig, newline: &str) -> String {
    if is_template_project(config) {
        return [
            "# Documentation".to_string(),
            String::new(),
            format!(
                "`{}` ships one CLI across Cargo, npm, and PyPI without maintaining three product implementations.",
                config.packages.command
            ),
            String::new(),
            "## Canonical Path".to_string(),
            String::new(),
            "- [Architecture](./architecture.md)".to_string(),
            "- [Agent Operations](./agent-operations.md)".to_string(),
            "- [Adoption Guide](./customizing-the-template.md)".to_string(),
            "- [Testing](./testing.md)".to_string(),
            "- [Releases](./releases.md)".to_string(),
            String::new(),
            "## Reference / Reports".to_string(),
            String::new(),
            "- [Live E2E](./live-e2e.md)".to_string(),
            "- [Package Size Report](./package-size-report.md)".to_string(),
            "- [JavaScript Package Size Report](./javascript-package-size-report.md)".to_string(),
            "- [Python Package Size Report](./python-package-size-report.md)".to_string(),
            String::new(),
            "## Decision Records".to_string(),
            String::new(),
            "- [ADR Index](./adrs/README.md)".to_string(),
            "- [ADR 0001: Rust Core, Thin Wrappers](./adrs/0001-rust-core-thin-wrappers.md)".to_string(),
            "- [ADR 0015: Agent-First Machine-Readable Repo Contract](./adrs/0015-agent-first-machine-readable-repo-contract.md)".to_string(),
            String::new(),
        ]
        .join(newline);
    }

    [
        "# Documentation".to_string(),
        String::new(),
        format!(
            "These docs describe the generated `{}` project and its local maintenance contract.",
            config.packages.command
        ),
        String::new(),
        "## Start Here".to_string(),
        String::new(),
        "- [Architecture](./architecture.md)".to_string(),
        "- [Agent Operations](./agent-operations.md)".to_string(),
        "- [Testing](./testing.md)".to_string(),
        "- [Releases](./releases.md)".to_string(),
        String::new(),
        "## Notes".to_string(),
        String::new(),
        "- The Rust core is the product source of truth.".to_string(),
        "- The JavaScript and Python packages are delivery adapters for the same CLI.".to_string(),
        "- Runtime package folders under `wrapper-js/platform-packages/ossplate-*` remain scaffold internals and are not the public npm package names.".to_string(),
        String::new(),
    ]
    .join(newline)
}

fn render_releases_doc_with_newlines(config: &ToolConfig, newline: &str) -> String {
    let runtime_package_names = ["darwin-arm64", "darwin-x64", "linux-x64", "windows-x64"]
        .into_iter()
        .map(|suffix| format!("- `{}-{suffix}`", config.packages.npm_package))
        .collect::<Vec<_>>();

    if is_template_project(config) {
        return [
            "# Releases".to_string(),
            String::new(),
            format!(
                "Use this guide when cutting or recovering an `{}` release.",
                config.packages.command
            ),
            String::new(),
            "## Current Registry Model".to_string(),
            String::new(),
            format!("- crates.io publishes `{}`", config.packages.rust_crate),
            format!(
                "- npm publishes `{}` plus runtime packages",
                config.packages.npm_package
            ),
            format!("- PyPI publishes `{}`", config.packages.python_package),
            format!("- the CLI name is `{}`", config.packages.command),
            String::new(),
            "Current npm runtime package names for this repository are:".to_string(),
            String::new(),
            runtime_package_names.join(newline),
            String::new(),
            "Run the full gate before releasing:".to_string(),
            String::new(),
            "```bash".to_string(),
            "./scripts/verify.sh".to_string(),
            "cargo run --manifest-path core-rs/Cargo.toml -- publish --plan --json".to_string(),
            "cargo run --manifest-path core-rs/Cargo.toml -- verify --json".to_string(),
            "```".to_string(),
            String::new(),
        ]
        .join(newline);
    }

    [
        "# Releases".to_string(),
        String::new(),
        format!(
            "Use this guide when cutting or recovering a `{}` release.",
            config.packages.command
        ),
        String::new(),
        "## Current Registry Model".to_string(),
        String::new(),
        format!("- crates.io publishes `{}`", config.packages.rust_crate),
        format!("- npm publishes `{}` plus runtime packages", config.packages.npm_package),
        format!("- PyPI publishes `{}`", config.packages.python_package),
        format!("- the CLI name is `{}`", config.packages.command),
        String::new(),
        "Current npm runtime package names are derived from the top-level npm package:".to_string(),
        String::new(),
        runtime_package_names.join(newline),
        String::new(),
        "`win32-x64` remains the internal target identifier. Runtime package folder names under `wrapper-js/platform-packages/ossplate-*` are scaffold internals, not the public npm package naming contract.".to_string(),
        String::new(),
        "## Required Preflight".to_string(),
        String::new(),
        "```bash".to_string(),
        "./scripts/verify.sh".to_string(),
        "cargo run --manifest-path core-rs/Cargo.toml -- publish --plan --json".to_string(),
        "cargo run --manifest-path core-rs/Cargo.toml -- verify --json".to_string(),
        "```".to_string(),
        String::new(),
        "## Local Operator Publish".to_string(),
        String::new(),
        "```bash".to_string(),
        "cargo run --manifest-path core-rs/Cargo.toml -- publish --dry-run".to_string(),
        "cargo run --manifest-path core-rs/Cargo.toml -- publish --registry npm --skip-existing".to_string(),
        "cargo run --manifest-path core-rs/Cargo.toml -- publish --registry pypi".to_string(),
        "```".to_string(),
        String::new(),
        "## Release Flow".to_string(),
        String::new(),
        "1. Push releasable work to `main`.".to_string(),
        "2. Let CI pass on that commit.".to_string(),
        "3. Run the project release workflow or the equivalent local operator flow.".to_string(),
        "4. Only after downstream publish success should a GitHub release be created.".to_string(),
        "5. After npm publish settles, keep `wrapper-js/package-lock.json` aligned with the released version state on `main`.".to_string(),
        String::new(),
    ]
    .join(newline)
}

fn relative_docs_link(file_name: &str) -> String {
    format!("../docs/{file_name}")
}

pub(crate) fn github_raw_url(repository: &str, branch: &str, path: &str) -> Result<String> {
    let repo = github_repository_path(repository)?;
    Ok(format!(
        "https://raw.githubusercontent.com/{repo}/{branch}/{path}"
    ))
}

pub(crate) fn github_blob_url(repository: &str, branch: &str, path: &str) -> Result<String> {
    let repo = github_repository_path(repository)?;
    Ok(format!("https://github.com/{repo}/blob/{branch}/{path}"))
}

fn github_repository_path(repository: &str) -> Result<String> {
    let trimmed = repository.trim_end_matches('/');
    if let Some(rest) = trimmed.strip_prefix("https://github.com/") {
        return Ok(rest.to_string());
    }
    if let Some(rest) = trimmed.strip_prefix("git@github.com:") {
        return Ok(rest.to_string());
    }
    bail!(
        "unsupported repository URL for published README links: {}",
        repository
    )
}

fn validate_workflow_name(
    path: &str,
    expected_name: &str,
    content: &str,
) -> Result<Vec<ValidationIssue>> {
    let expected = format!("name: {}\n", expected_name);
    let actual = extract_marked_section(content, WORKFLOW_NAME_START, WORKFLOW_NAME_END)?;
    if normalize_newlines(&actual) == normalize_newlines(&expected) {
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

pub(crate) fn issue(
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

pub(crate) fn format_human_issues(header: &str, issues: &[ValidationIssue]) -> String {
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
    Ok(normalize_newlines(content[section_start..end_index].trim_matches(['\r', '\n'])) + "\n")
}

fn replace_marked_section(
    content: &str,
    start: &str,
    end: &str,
    replacement: &str,
) -> Result<String> {
    let newline = detect_newline_style(content);
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
    rendered.push_str(newline);
    rendered.push_str(&normalize_newlines(replacement).replace('\n', newline));
    if !rendered.ends_with(newline) {
        rendered.push_str(newline);
    }
    rendered.push_str(&content[end_index..]);
    Ok(rendered)
}

pub(crate) fn normalize_newlines(value: &str) -> String {
    value.replace("\r\n", "\n")
}

fn detect_newline_style(content: &str) -> &str {
    if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}
