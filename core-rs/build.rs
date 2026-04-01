use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let template_root = resolve_template_root(&manifest_dir);
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    let mut entries = Vec::new();
    collect_template_entries(&template_root, &template_root, "", &mut entries);
    collect_template_entries(
        &template_root,
        &template_root,
        "core-rs/embedded-template-root/",
        &mut entries,
    );
    collect_core_entries(&manifest_dir, "core-rs/", &mut entries);
    collect_core_entries(
        &manifest_dir,
        "core-rs/embedded-template-root/core-rs/",
        &mut entries,
    );
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut generated =
        String::from("pub(crate) const EMBEDDED_TEMPLATE_FILES: &[(&str, &[u8])] = &[\n");
    for (target_path, absolute_path) in entries {
        let escaped_target_path = escape_rust_string(&target_path);
        let escaped_absolute_path = escape_rust_string(
            absolute_path
                .to_str()
                .expect("repo paths must be valid UTF-8 for build embedding"),
        );
        generated.push_str(&format!(
            "    (\"{escaped_target_path}\", include_bytes!(\"{escaped_absolute_path}\")),\n"
        ));
    }
    generated.push_str("];\n");

    fs::write(out_dir.join("embedded_template.rs"), generated)
        .expect("failed to write embedded template manifest");
}

fn resolve_template_root(manifest_dir: &Path) -> PathBuf {
    let generated_root = manifest_dir.join("generated-embedded-template-root");
    if let Some(repo_root) = detect_template_repo_root(manifest_dir) {
        println!(
            "cargo:rerun-if-changed={}",
            repo_root.join("ossplate.toml").display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            repo_root.join("scaffold-payload.json").display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            repo_root.join("source-checkout.json").display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            repo_root
                .join("scripts/stage-embedded-template.mjs")
                .display()
        );
    }

    if generated_root.is_dir() {
        return generated_root;
    }

    let scaffold_root = manifest_dir.join("embedded-template-root");
    if scaffold_root.is_dir() {
        return scaffold_root;
    }

    panic!(
        "missing embedded template payload. Run `node scripts/stage-distribution-assets.mjs embedded-template` from the repo root before building or packaging core-rs, or restore core-rs/embedded-template-root in a scaffolded repo."
    );
}

fn detect_template_repo_root(manifest_dir: &Path) -> Option<PathBuf> {
    let repo_root = manifest_dir.parent()?;
    let scaffold_manifest = repo_root.join("scaffold-payload.json");
    let stage_script = repo_root.join("scripts/stage-embedded-template.mjs");
    if scaffold_manifest.is_file() && stage_script.is_file() {
        Some(repo_root.to_path_buf())
    } else {
        None
    }
}

fn collect_template_entries(
    root: &Path,
    current: &Path,
    target_prefix: &str,
    entries: &mut Vec<(String, PathBuf)>,
) {
    for entry in fs::read_dir(current)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", current.display()))
    {
        let entry = entry.unwrap_or_else(|err| panic!("failed to read dir entry: {err}"));
        let path = entry.path();
        let file_type = entry
            .file_type()
            .unwrap_or_else(|err| panic!("failed to stat {}: {err}", path.display()));
        if file_type.is_dir() {
            collect_template_entries(root, &path, target_prefix, entries);
        } else if file_type.is_file() {
            println!("cargo:rerun-if-changed={}", path.display());
            let relative_path = path
                .strip_prefix(root)
                .unwrap_or_else(|err| {
                    panic!(
                        "failed to strip {} prefix from {}: {err}",
                        root.display(),
                        path.display()
                    )
                })
                .to_string_lossy()
                .replace('\\', "/");
            entries.push((format!("{target_prefix}{relative_path}"), path));
        }
    }
}

fn collect_core_entries(
    manifest_dir: &Path,
    target_prefix: &str,
    entries: &mut Vec<(String, PathBuf)>,
) {
    for relative_path in [
        "Cargo.toml",
        "Cargo.lock",
        "build.rs",
        "runtime-targets.json",
        "source-checkout.json",
    ] {
        let path = manifest_dir.join(relative_path);
        println!("cargo:rerun-if-changed={}", path.display());
        entries.push((format!("{target_prefix}{relative_path}"), path));
    }

    let src_root = manifest_dir.join("src");
    collect_core_src_entries(&src_root, &src_root, target_prefix, entries);
}

fn collect_core_src_entries(
    root: &Path,
    current: &Path,
    target_prefix: &str,
    entries: &mut Vec<(String, PathBuf)>,
) {
    for entry in fs::read_dir(current)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", current.display()))
    {
        let entry = entry.unwrap_or_else(|err| panic!("failed to read dir entry: {err}"));
        let path = entry.path();
        let file_type = entry
            .file_type()
            .unwrap_or_else(|err| panic!("failed to stat {}: {err}", path.display()));
        if file_type.is_dir() {
            collect_core_src_entries(root, &path, target_prefix, entries);
        } else if file_type.is_file() {
            println!("cargo:rerun-if-changed={}", path.display());
            let relative_path = path
                .strip_prefix(root)
                .unwrap_or_else(|err| {
                    panic!(
                        "failed to strip {} prefix from {}: {err}",
                        root.display(),
                        path.display()
                    )
                })
                .to_string_lossy()
                .replace('\\', "/");
            entries.push((format!("{target_prefix}src/{relative_path}"), path));
        }
    }
}

fn escape_rust_string(value: &str) -> String {
    value.chars().flat_map(char::escape_default).collect()
}
