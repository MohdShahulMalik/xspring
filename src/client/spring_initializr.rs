use std::{io::Cursor, path::PathBuf, time::Instant};

use anyhow::{anyhow, Context, Result};
use reqwest::header::{ACCEPT, ETAG, IF_NONE_MATCH, USER_AGENT};
use reqwest::{Client, StatusCode};
use tracing::{debug, info, warn};
use zip::ZipArchive;

use crate::client::cache::MetadataCache;
use crate::client::http::{http_client, USER_AGENT as CLI_USER_AGENT};
use crate::models::spring::{InitializrMetadata, QueryParam};

/// Default base URL of the Spring Initializr API.
const DEFAULT_BASE_URL: &str = "https://start.spring.io";

/// Base URL of the Spring Initializr API.
///
/// Override with `XSPRING_API_URL` (useful for mirrors and for testing).
pub fn base_url() -> String {
    crate::util::env_non_empty("XSPRING_API_URL")
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
        .trim_end_matches('/')
        .to_string()
}

/// URL of the metadata document.
fn metadata_url() -> String {
    format!("{}/", base_url())
}

/// URL of the generated project archive.
fn starter_url() -> String {
    format!("{}/starter.zip", base_url())
}

/// Fetches the Initializr metadata document.
///
/// The document is served from the local cache while it is fresh; a stale
/// entry is revalidated with a conditional request, which normally comes back
/// as `304 Not Modified`. If the network is unreachable, a stale cache entry
/// is used as a fallback so the CLI still works offline.
pub async fn get_metadata() -> Result<InitializrMetadata> {
    get_metadata_with(
        MetadataCache::load(),
        &metadata_url(),
        crate::client::cache::ttl_seconds(),
        http_client(),
    )
    .await
}

/// [`get_metadata`] with every dependency injected, so the caching behaviour
/// can be exercised without touching the network or the environment.
pub async fn get_metadata_with(
    mut cache: MetadataCache,
    url: &str,
    ttl_seconds: u64,
    client: &Client,
) -> Result<InitializrMetadata> {
    let started = Instant::now();
    let now = crate::util::now_secs();

    // Fast path: the cached document is still fresh, no network at all.
    if let Some(entry) = cache.entry() {
        if entry.is_fresh(now, ttl_seconds) {
            info!(
                "metadata: served from cache (age {}s, ttl {}s), no network request",
                entry.age_seconds(now),
                ttl_seconds
            );
            return decode(&entry.body);
        }

        debug!(
            "metadata: cache is stale (age {}s, ttl {}s), revalidating",
            entry.age_seconds(now),
            ttl_seconds
        );
    } else {
        debug!("metadata: no cache entry, fetching from the network");
    }

    match fetch(&mut cache, url, now, client).await {
        Ok(metadata) => {
            info!("metadata: ready in {:?}", started.elapsed());
            Ok(metadata)
        }
        Err(err) => {
            // The network failed: a stale cache entry is better than nothing.
            let Some(entry) = cache.entry() else {
                return Err(err);
            };

            match decode(&entry.body) {
                Ok(metadata) => {
                    warn!(
                        "metadata: fetch failed ({err:#}), falling back to the cached copy (age {}s)",
                        entry.age_seconds(now)
                    );
                    Ok(metadata)
                }
                Err(cache_err) => {
                    warn!(
                        "metadata: fetch failed ({err:#}) and the cached copy is unusable ({cache_err:#})"
                    );
                    Err(err)
                }
            }
        }
    }
}

/// Downloads (or revalidates) the metadata document and updates the cache.
async fn fetch(
    cache: &mut MetadataCache,
    url: &str,
    now: u64,
    client: &Client,
) -> Result<InitializrMetadata> {
    let mut request = client
        .get(url)
        .header(ACCEPT, "application/json")
        .header(USER_AGENT, CLI_USER_AGENT);

    if let Some(etag) = cache.entry().and_then(|entry| entry.etag.clone()) {
        debug!("metadata: revalidating with If-None-Match: {etag}");
        request = request.header(IF_NONE_MATCH, etag);
    }

    let started = Instant::now();
    let response = match request.send().await {
        Ok(response) => response,
        Err(err) if err.is_timeout() => {
            return Err(anyhow!(
                "Timed out after {}s while fetching metadata from {url}. The Spring \
                 Initializr may be slow or unreachable — retry, or raise \
                 XSPRING_HTTP_TIMEOUT.",
                crate::client::http::request_timeout_seconds()
            ));
        }
        Err(err) => {
            return Err(err).with_context(|| "Failed to fetch metadata from spring api");
        }
    };

    let status = response.status();
    let etag = response
        .headers()
        .get(ETAG)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);

    debug!(
        "metadata: server answered {status} in {:?}",
        started.elapsed()
    );

    // `304` means the cached document is still current.
    if status == StatusCode::NOT_MODIFIED {
        let Some(body) = cache.entry().map(|entry| entry.body.clone()) else {
            return Err(anyhow!(
                "Server returned 304 Not Modified but no cached metadata is available"
            ));
        };

        info!("metadata: 304 Not Modified, the cached copy is still current");
        if let Err(err) = cache.refresh(now) {
            warn!("metadata: could not refresh the cache timestamp: {err}");
        }

        return decode(&body);
    }

    if !status.is_success() {
        return Err(anyhow!("Failed to fetch metadata {}", status));
    }

    let body = response
        .text()
        .await
        .with_context(|| "Failed to read the metadata response body")?;

    debug!("metadata: received {} bytes", body.len());

    // Decode *before* storing. A document we cannot parse must never replace a
    // cached copy that does parse: if upstream changes the schema, the old
    // (working) document stays in the cache and the CLI keeps running.
    let metadata = decode(&body)?;

    match cache.store(body.clone(), etag, now) {
        Ok(()) => debug!("metadata: cache updated at {}", cache.path().display()),
        Err(err) => warn!("metadata: could not update the cache: {err}"),
    }

    Ok(metadata)
}

/// Parses the metadata document.
fn decode(body: &str) -> Result<InitializrMetadata> {
    serde_json::from_str(body)
        .with_context(|| "Failed to deserialize the metadata fetched from the spring api")
}

/// Downloads the generated project and extracts it into `out_dir`.
pub async fn generate_project(params: QueryParam, out_dir: PathBuf) -> Result<()> {
    let client = http_client();
    let started = Instant::now();
    let url = starter_url();

    debug!("project: requesting {} for {:?}", url, params);

    let response = match client.get(&url).query(&params).send().await {
        Ok(response) => response,
        Err(err) if err.is_timeout() => {
            return Err(anyhow!(
                "Timed out after {}s while downloading the generated project from {url}. \
                 The Spring Initializr may be slow or unreachable — retry, or raise \
                 XSPRING_HTTP_TIMEOUT.",
                crate::client::http::request_timeout_seconds()
            ));
        }
        Err(err) => {
            return Err(err).with_context(|| "Failed to download the generated project from spring api");
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Could not read error body".to_string());
        return Err(anyhow!(
            "Failed to generate project with status: {}. Body: {}",
            status,
            body
        ));
    }

    let project_bytes = response
        .bytes()
        .await
        .with_context(|| "Failed to read the generated project archive")?;

    debug!(
        "project: downloaded {} bytes in {:?}",
        project_bytes.len(),
        started.elapsed()
    );

    let cursor = Cursor::new(project_bytes);
    let mut archive = ZipArchive::new(cursor)?;
    archive.extract(&out_dir)?;

    let project_dir = out_dir.join(&params.base_dir);
    info!(
        "project: extracted to {:?} in {:?}",
        project_dir,
        started.elapsed()
    );
    println!(
        "Successfully Generated a Spring Boot project folder at {:?}",
        project_dir
    );

    Ok(())
}
