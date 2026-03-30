use crate::config::ToolConfig;
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
    let expected = render_root_readme_identity(config);
    let actual = extract_marked_section(content, README_IDENTITY_START, README_IDENTITY_END)?;
    if normalize_newlines(&actual) == normalize_newlines(&expected) {
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

pub(crate) fn sync_root_readme(config: &ToolConfig, content: &str) -> Result<String> {
    replace_marked_section(
        content,
        README_IDENTITY_START,
        README_IDENTITY_END,
        &render_root_readme_identity(config),
    )
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

pub(crate) fn render_root_readme_identity(config: &ToolConfig) -> String {
    format!(
        "# {}\n\n{}\n",
        config.project.name, config.project.description
    )
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
