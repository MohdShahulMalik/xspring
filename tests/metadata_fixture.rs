use xspring::models::spring::InitializrMetadata;

/// Real start.spring.io response (fetched 2026-10-19). Serializing our
/// structs against it catches upstream API drift before users do — e.g. the
/// day `name`/`description` stopped shipping a `default` field.
const METADATA: &str = include_str!("fixtures/metadata.json");

#[test]
fn deserializes_live_api_metadata() {
    let metadata: InitializrMetadata =
        serde_json::from_str(METADATA).expect("metadata must deserialize");

    assert_eq!(metadata.name.default, None);
    assert_eq!(metadata.description.default, None);

    assert_eq!(metadata.boot_version.default, "4.1.1.RELEASE");
    assert_eq!(metadata.java_version.default, "17");
    assert_eq!(metadata.language.default, "java");
    assert_eq!(metadata.packaging.default, "jar");
    assert_eq!(metadata.project_type.default, "gradle-project");

    assert!(!metadata.dependencies.values.is_empty());
    assert!(!metadata.boot_version.values.is_empty());
    for category in &metadata.dependencies.values {
        assert!(!category.values.is_empty(), "every category has values");
    }
}
