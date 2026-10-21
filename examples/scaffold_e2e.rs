//! Downloads a project from the real Spring Initializr and runs the
//! scaffolding step on it, so the whole flow can be checked end to end.
//!
//! Usage: cargo run --example scaffold_e2e -- <output-dir> [--no-git]

use std::path::PathBuf;

use xspring::client::spring_initializr::generate_project;
use xspring::models::spring::QueryParam;
use xspring::scaffold::finalize;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);

    let out_dir: PathBuf = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("xspring-e2e"));

    let init_git = !args.any(|arg| arg == "--no-git" || arg == "-ng");

    let params = QueryParam {
        project_type: "maven-project".to_string(),
        language: "java".to_string(),
        boot_version: "4.0.0".to_string(),
        group_id: "com.example".to_string(),
        artifact_id: "demo".to_string(),
        name: "demo".to_string(),
        description: "demo project".to_string(),
        packaging: "jar".to_string(),
        java_version: "17".to_string(),
        dependencies: "web".to_string(),
        base_dir: "demo".to_string(),
    };

    let project_dir = generate_project(params, out_dir).await?;
    finalize(&project_dir, init_git)?;

    println!("project dir: {}", project_dir.display());

    Ok(())
}
