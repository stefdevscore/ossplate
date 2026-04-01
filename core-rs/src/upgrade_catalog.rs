use anyhow::Result;
use std::path::Path;

use crate::scaffold::upgrade_scaffold_from;
use crate::scaffold_manifest::required_source_paths;

#[derive(Debug, Clone)]
pub(crate) struct VersionSpec {
    pub(crate) version: u64,
    pub(crate) fingerprint: VersionFingerprint,
    pub(crate) migration_from_previous: Option<MigrationDefinition>,
}

#[derive(Debug, Clone)]
pub(crate) struct MigrationDefinition {
    pub(crate) from_version: u64,
    pub(crate) to_version: u64,
    pub(crate) apply: fn(&Path, &Path) -> Result<()>,
}

impl MigrationDefinition {
    pub(crate) fn label(&self) -> String {
        format!("{}->{}", self.from_version, self.to_version)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct VersionFingerprint {
    pub(crate) required_paths: Vec<String>,
    pub(crate) forbidden_paths: Vec<String>,
}

impl VersionFingerprint {
    pub(crate) fn matches(&self, root: &Path) -> bool {
        self.required_paths
            .iter()
            .all(|path| root.join(path).exists())
            && self
                .forbidden_paths
                .iter()
                .all(|path| !root.join(path).exists())
    }
}

const VERSION_1_REQUIRED_PATHS: &[&str] = &[
    "ossplate.toml",
    "scaffold-payload.json",
    "README.md",
    "core-rs/Cargo.toml",
    "core-rs/src/config.rs",
    "core-rs/src/main.rs",
    "core-rs/src/main_tests.rs",
    "core-rs/src/test_support.rs",
    "core-rs/src/output.rs",
    "core-rs/src/release.rs",
    "core-rs/src/scaffold.rs",
    "core-rs/src/scaffold_manifest.rs",
    "core-rs/src/scaffold/identity_application.rs",
    "core-rs/src/scaffold/projection.rs",
    "core-rs/src/scaffold/template_root.rs",
    "core-rs/src/source_checkout.rs",
    "core-rs/src/sync.rs",
    "core-rs/src/sync/metadata.rs",
    "core-rs/src/sync/text.rs",
    "scripts/stage-distribution-assets.mjs",
    "wrapper-js/package.json",
    "wrapper-py/pyproject.toml",
];

const VERSION_1_FORBIDDEN_PATHS: &[&str] = &[
    "core-rs/build.rs",
    "core-rs/src/embedded_template.rs",
    "core-rs/src/upgrade.rs",
    "core-rs/src/upgrade_catalog.rs",
    "core-rs/src/verify.rs",
    "scripts/stage-embedded-template.mjs",
    "scripts/package-js.mjs",
];

const VERSION_2_REQUIRED_PATHS: &[&str] = &[
    "ossplate.toml",
    "scaffold-payload.json",
    "README.md",
    "core-rs/Cargo.toml",
    "core-rs/build.rs",
    "core-rs/src/config.rs",
    "core-rs/src/embedded_template.rs",
    "core-rs/src/main.rs",
    "core-rs/src/main_tests.rs",
    "core-rs/src/test_support.rs",
    "core-rs/src/output.rs",
    "core-rs/src/release.rs",
    "core-rs/src/scaffold.rs",
    "core-rs/src/scaffold_manifest.rs",
    "core-rs/src/verify.rs",
    "core-rs/src/scaffold/identity_application.rs",
    "core-rs/src/scaffold/projection.rs",
    "core-rs/src/scaffold/template_root.rs",
    "core-rs/src/source_checkout.rs",
    "core-rs/src/sync.rs",
    "core-rs/src/sync/metadata.rs",
    "core-rs/src/sync/text.rs",
    "core-rs/src/upgrade.rs",
    "scripts/stage-distribution-assets.mjs",
    "scripts/stage-embedded-template.mjs",
    "scripts/package-js.mjs",
    "wrapper-js/package.json",
    "wrapper-py/pyproject.toml",
];

const VERSION_2_FORBIDDEN_PATHS: &[&str] = &["core-rs/src/upgrade_catalog.rs"];

pub(crate) fn authored_versions() -> Vec<VersionSpec> {
    vec![
        VersionSpec {
            version: 1,
            fingerprint: VersionFingerprint {
                required_paths: VERSION_1_REQUIRED_PATHS
                    .iter()
                    .map(|path| path.to_string())
                    .collect(),
                forbidden_paths: VERSION_1_FORBIDDEN_PATHS
                    .iter()
                    .map(|path| path.to_string())
                    .collect(),
            },
            migration_from_previous: None,
        },
        VersionSpec {
            version: 2,
            fingerprint: VersionFingerprint {
                required_paths: VERSION_2_REQUIRED_PATHS
                    .iter()
                    .map(|path| path.to_string())
                    .collect(),
                forbidden_paths: VERSION_2_FORBIDDEN_PATHS
                    .iter()
                    .map(|path| path.to_string())
                    .collect(),
            },
            migration_from_previous: Some(MigrationDefinition {
                from_version: 1,
                to_version: 2,
                apply: apply_scaffold_upgrade,
            }),
        },
        VersionSpec {
            version: 3,
            fingerprint: VersionFingerprint {
                required_paths: required_source_paths(),
                forbidden_paths: Vec::new(),
            },
            migration_from_previous: Some(MigrationDefinition {
                from_version: 2,
                to_version: 3,
                apply: apply_scaffold_upgrade,
            }),
        },
    ]
}

fn apply_scaffold_upgrade(source_root: &Path, target_root: &Path) -> Result<()> {
    upgrade_scaffold_from(source_root, target_root)
}
