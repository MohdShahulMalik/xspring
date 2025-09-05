use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InitializrMetadata {
    pub dependencies: Dependencies,
    pub boot_version: MetadataValue,
    pub java_version: MetadataValue,
    pub language: MetadataValue,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Dependencies {
    #[serde(rename = "type")]
    pub type_field: String,
    pub values: Vec<Dependency>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
    pub name: String,
    pub values: Vec<DependencyDetails>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DependencyDetails {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MetadataValue {
    #[serde(rename = "type")]
    pub type_field: String,
    pub default: String,
    pub values: Vec<Value>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Value {
    pub id: String,
    pub name: String,
}
