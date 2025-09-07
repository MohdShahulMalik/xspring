use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InitializrMetadata {
    pub dependencies: Dependencies,
    pub boot_version: MetadataValue,
    pub java_version: MetadataValue,
    pub language: MetadataValue,
    pub packaging: MetadataValue,
    #[serde(rename = "type")]
    pub project_type: MetadataValue,
    pub name: DefaultText,
    pub description: DefaultText,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QueryParam {
    pub project_type: String,
    pub language: String,
    pub boot_version: String,
    pub group_id: String,
    pub artifact_id: String,
    pub name: String,
    pub description: String,
    pub packaging: String,
    pub java_version: String,
    pub dependencies: String,
    pub base_dir: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Dependencies {
    pub values: Vec<DependencyCategories>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DependencyCategories {
    pub name: String,
    pub values: Vec<Value>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MetadataValue {
    pub default: String,
    pub values: Vec<Value>,
}

#[derive(Deserialize, Debug)]
pub struct DefaultText {
    pub default: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Value {
    pub id: String,
    pub name: String,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}
