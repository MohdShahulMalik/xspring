//! On-disk cache for the `start.spring.io` metadata document.
//!
//! `GET https://start.spring.io` answers with a ~75 KB JSON document that
//! describes every dependency, boot version, packaging option and so on.
//! Building that document is the slowest thing this CLI does. The server
//! needs roughly 300 ms for it and sends the response uncompressed.
//!
//! The document itself barely ever changes. The API sends
//! `cache-control: max-age=7200` and an `ETag` to say so.
//!
//! The last response is kept on disk and reused:
//!
//! * a **fresh** entry is served without touching the network at all;
//! * a **stale** entry is revalidated with a conditional request
//!   (`If-None-Match`), which normally answers `304 Not Modified` and costs
//!   one round trip instead of a full download;
//! * if the network is unreachable, a stale entry is used as a fallback so the
//!   CLI keeps working offline.
//!
//! ## Environment variables
//!
//! | Variable | Effect |
//! |---|---|
//! | `XSPRING_CACHE_DIR` | Directory holding the cache file. Default: `$XDG_CACHE_HOME/xspring`, else `$HOME/.cache/xspring`, else a temp directory. |
//! | `XSPRING_CACHE_TTL` | Seconds a cached document is served without revalidation. Default: `7200` (matches the server's `max-age`). `0` disables the fast path, so every run revalidates. |
//! | `XSPRING_CACHE` | Set to `off` to disable the cache entirely (no read, no write). |

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// File name of the cache entry inside the cache directory.
const CACHE_FILE: &str = "metadata.json";

/// How long a cached document is served without contacting the server.
///
/// Matches the `cache-control: max-age=7200` the Initializr sends.
pub const DEFAULT_TTL_SECONDS: u64 = 7200;

/// One cached metadata document plus the validator needed to revalidate it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// The `ETag` the server sent with the body, replayed as `If-None-Match`.
    #[serde(default)]
    pub etag: Option<String>,
    /// Unix timestamp (seconds) of the moment the body was received.
    pub fetched_at: u64,
    /// The raw metadata JSON, exactly as the server sent it.
    pub body: String,
}

impl CacheEntry {
    /// Seconds since the body was received.
    pub fn age_seconds(&self, now: u64) -> u64 {
        now.saturating_sub(self.fetched_at)
    }

    /// Whether the entry may be served without asking the server.
    pub fn is_fresh(&self, now: u64, ttl_seconds: u64) -> bool {
        self.age_seconds(now) < ttl_seconds
    }
}

/// The cache store: a single JSON document inside the cache directory.
pub struct MetadataCache {
    path: PathBuf,
    entry: Option<CacheEntry>,
    enabled: bool,
}

impl MetadataCache {
    /// Loads the cache from its default location.
    ///
    /// A corrupt cache file is discarded rather than propagated; the caller
    /// then simply refetches.
    pub fn load() -> Self {
        Self::open(cache_dir().join(CACHE_FILE), cache_enabled())
    }

    /// Opens the cache at an explicit path.
    pub fn open(path: PathBuf, enabled: bool) -> Self {
        let entry = if enabled {
            read_entry(&path)
        } else {
            debug!("cache: disabled via XSPRING_CACHE");
            None
        };

        MetadataCache {
            path,
            entry,
            enabled,
        }
    }

    /// The cached entry, if there is one.
    pub fn entry(&self) -> Option<&CacheEntry> {
        self.entry.as_ref()
    }

    /// Where the cache document lives.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Whether the cache is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Stores a freshly downloaded document.
    pub fn store(&mut self, body: String, etag: Option<String>, now: u64) -> io::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let entry = CacheEntry {
            etag,
            fetched_at: now,
            body,
        };
        write_entry(&self.path, &entry)?;
        self.entry = Some(entry);
        Ok(())
    }

    /// Marks the cached document as freshly validated (used after `304`).
    pub fn refresh(&mut self, now: u64) -> io::Result<()> {
        let Some(entry) = self.entry.as_mut() else {
            return Ok(());
        };

        entry.fetched_at = now;
        write_entry(&self.path, entry)
    }
}

/// Directory the cache document lives in.
pub fn cache_dir() -> PathBuf {
    if let Some(dir) = crate::util::env_non_empty("XSPRING_CACHE_DIR") {
        return PathBuf::from(dir);
    }

    if let Some(base) = crate::util::env_non_empty("XDG_CACHE_HOME") {
        return PathBuf::from(base).join("xspring");
    }

    if let Some(home) = crate::util::env_non_empty("HOME") {
        return PathBuf::from(home).join(".cache").join("xspring");
    }

    // No HOME-based location: prefer a user-private runtime directory over the
    // shared temp dir, so another local user cannot pre-seed the cache. Logs
    // wiped on reboot are an acceptable loss when there is no HOME anyway.
    if let Some(runtime) = crate::util::env_non_empty("XDG_RUNTIME_DIR") {
        return PathBuf::from(runtime).join("xspring");
    }

    std::env::temp_dir().join("xspring")
}

/// How long a cached document is served without revalidation, in seconds.
pub fn ttl_seconds() -> u64 {
    crate::util::env_non_empty("XSPRING_CACHE_TTL")
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_TTL_SECONDS)
}

/// Whether the cache is enabled at all.
pub fn cache_enabled() -> bool {
    match crate::util::env_non_empty("XSPRING_CACHE") {
        Some(value) => !matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "off" | "0" | "false" | "no"
        ),
        None => true,
    }
}

/// Reads and parses the cache document, discarding it when it is unusable.
fn read_entry(path: &Path) -> Option<CacheEntry> {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) => {
            if err.kind() != io::ErrorKind::NotFound {
                warn!("cache: could not read {}: {err}", path.display());
            }
            return None;
        }
    };

    match serde_json::from_str::<CacheEntry>(&raw) {
        Ok(entry) => {
            debug!(
                "cache: loaded {} ({} bytes, etag {:?})",
                path.display(),
                entry.body.len(),
                entry.etag
            );
            Some(entry)
        }
        Err(err) => {
            warn!(
                "cache: {} is corrupt ({err}), discarding it",
                path.display()
            );
            let _ = std::fs::remove_file(path);
            None
        }
    }
}

/// Writes the cache document atomically, so a crash mid-write can never leave
/// a half-written file behind.
fn write_entry(path: &Path, entry: &CacheEntry) -> io::Result<()> {
    let json =
        serde_json::to_vec(entry).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

    // Owner-only: on a multi-user system no other account can read the cached
    // document or tamper with it before the CLI revalidates it.
    crate::util::atomic_write_full(path, &json, Some(0o600))
}
