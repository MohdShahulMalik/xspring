use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use crate::models::spring::InitializrMetadata;

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
