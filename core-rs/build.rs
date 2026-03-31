use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let template_root = manifest_dir.join("embedded-template-root");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    let mut entries = Vec::new();
    collect_template_entries(&template_root, &template_root, "", &mut entries);
    collect_template_entries(
        &template_root,
        &template_root,
        "core-rs/embedded-template-root/",
        &mut entries,
    );
    collect_core_entries(&manifest_dir, &mut entries);
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

fn collect_core_entries(manifest_dir: &Path, entries: &mut Vec<(String, PathBuf)>) {
    for relative_path in [
        "Cargo.toml",
        "Cargo.lock",
        "build.rs",
        "runtime-targets.json",
        "source-checkout.json",
    ] {
        let path = manifest_dir.join(relative_path);
        println!("cargo:rerun-if-changed={}", path.display());
        entries.push((format!("core-rs/{relative_path}"), path));
    }

    let src_root = manifest_dir.join("src");
    collect_core_src_entries(&src_root, &src_root, entries);
}

fn collect_core_src_entries(root: &Path, current: &Path, entries: &mut Vec<(String, PathBuf)>) {
    for entry in fs::read_dir(current)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", current.display()))
    {
        let entry = entry.unwrap_or_else(|err| panic!("failed to read dir entry: {err}"));
        let path = entry.path();
        let file_type = entry
            .file_type()
            .unwrap_or_else(|err| panic!("failed to stat {}: {err}", path.display()));
        if file_type.is_dir() {
            collect_core_src_entries(root, &path, entries);
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
            entries.push((format!("core-rs/src/{relative_path}"), path));
        }
    }
}

fn escape_rust_string(value: &str) -> String {
    value.chars().flat_map(char::escape_default).collect()
}
