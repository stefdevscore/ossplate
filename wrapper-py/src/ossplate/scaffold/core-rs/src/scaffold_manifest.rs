use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct ScaffoldManifest {
    #[serde(rename = "requiredPaths")]
    required_source_paths: Vec<String>,
}

pub(crate) fn required_source_paths() -> Vec<String> {
    serde_json::from_str::<ScaffoldManifest>(include_str!("../source-checkout.json"))
        .expect("source-checkout.json must parse")
        .required_source_paths
}
