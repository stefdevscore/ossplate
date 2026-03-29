use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct ScaffoldManifest {
    #[serde(rename = "requiredSourcePaths")]
    required_source_paths: Vec<String>,
}

pub(crate) fn required_source_paths() -> Vec<String> {
    serde_json::from_str::<ScaffoldManifest>(include_str!("../../scaffold-manifest.json"))
        .expect("scaffold-manifest.json must parse")
        .required_source_paths
}
