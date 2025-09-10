use std::{io::Cursor, path::PathBuf};
use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use tracing::info;
use zip::ZipArchive;
use crate::models::spring::{InitializrMetadata, QueryParam};

pub async fn get_metadata() -> Result<InitializrMetadata>{
    let client = Client::new();

    let response = client
        .get("https://start.spring.io")
        .header("Accept", "application/json")
        .header("User-Agent", "XSpring CLI tool")
        .send()
        .await.with_context(|| "Failed to fetch metadata from spring api")?;

    if !response.status().is_success() {
        return Err(anyhow!("Failed to fetch metadata {}", response.status()))
    }

    let metadata = response.json::<InitializrMetadata>().await.with_context(|| "Failed to deserialize the metadata fetched from the spring api")?;

    Ok(metadata)
}

pub async fn generate_project(params: QueryParam, out_dir: PathBuf) -> Result<()> {
    let client = Client::new();

    let response = client
        .get("https://start.spring.io/starter.zip")
        .query(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_else(|_| "Could not read error body".to_string());
        return Err(anyhow!("Failed to generate project with status: {}. Body: {}", status, body));
    }

    let project_bytes = response.bytes().await?;
    let cursor = Cursor::new(project_bytes);
    let mut archive = ZipArchive::new(cursor)?;
    archive.extract(&out_dir)?;

    info!("Successfully Generated a Spring Boot project file at {:?}", out_dir);

    Ok(())
}
