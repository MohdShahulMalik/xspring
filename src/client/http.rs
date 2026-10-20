//! Shared HTTP client.
//!
//! Building a `reqwest::Client` is cheap (tens of microseconds), but every
//! client owns its own connection pool, so a fresh client per request also
//! throws away the TCP/TLS handshake that was already paid for. The client is
//! therefore built exactly once per process and reused for every request.
//!
//! The client also carries the request timeouts. Without them a hung
//! Initializr server blocks the CLI forever.

use std::sync::OnceLock;
use std::time::Duration;

use reqwest::Client;

/// Default overall timeout for a single request, in seconds.
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 30;

/// Default timeout for establishing a connection, in seconds.
pub const DEFAULT_CONNECT_TIMEOUT_SECONDS: u64 = 10;

/// Identifies the CLI to the server.
pub const USER_AGENT: &str = "XSpring CLI tool";

static CLIENT: OnceLock<Client> = OnceLock::new();

/// The process-wide HTTP client.
///
/// Built on first use and reused afterwards, so repeated requests in one run
/// share a connection pool.
pub fn http_client() -> &'static Client {
    CLIENT.get_or_init(|| {
        Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(Duration::from_secs(connect_timeout_seconds()))
            .timeout(Duration::from_secs(request_timeout_seconds()))
            .build()
            .expect("failed to build the HTTP client")
    })
}

/// Overall timeout for a single request, in seconds.
///
/// Override with `XSPRING_HTTP_TIMEOUT`.
pub fn request_timeout_seconds() -> u64 {
    crate::util::env_non_empty("XSPRING_HTTP_TIMEOUT")
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|seconds| *seconds > 0)
        .unwrap_or(DEFAULT_TIMEOUT_SECONDS)
}

/// Timeout for establishing a connection, in seconds.
///
/// Override with `XSPRING_HTTP_CONNECT_TIMEOUT`.
pub fn connect_timeout_seconds() -> u64 {
    crate::util::env_non_empty("XSPRING_HTTP_CONNECT_TIMEOUT")
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|seconds| *seconds > 0)
        .unwrap_or(DEFAULT_CONNECT_TIMEOUT_SECONDS)
}
