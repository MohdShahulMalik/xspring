//! Phase-level breakdown of where a single `xspring` invocation spends time.
//!
//! Run with:
//!   cargo run --release --example profile_phases
//!
//! It separates:
//!   1. client construction (TLS config + connection pool setup)
//!   2. DNS + TCP + TLS handshake + server think time (time to first byte)
//!   3. body download
//!   4. JSON deserialization
//!
//! and then shows what the metadata cache does on top of that.
//!
//! For a flame graph of the whole process use `samply` or `cargo-flamegraph`:
//!   cargo install samply
//!   samply record -- ./target/release/xspring list --java

use std::time::Instant;

use xspring::client::http::http_client;
use xspring::client::spring_initializr::base_url;
use xspring::models::spring::InitializrMetadata;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let program_start = Instant::now();
    let url = format!("{}/", base_url());

    println!("== xspring phase profile ==");
    println!("url: {url}\n");

    // ------------------------------------------------------------------
    // 1. Client construction
    // ------------------------------------------------------------------
    let t = Instant::now();
    let client = http_client();
    let client_build = t.elapsed();

    // ------------------------------------------------------------------
    // 2. Request: send() returns once the response *headers* are available.
    //    This is where DNS + TCP + TLS + server processing live.
    // ------------------------------------------------------------------
    let t = Instant::now();
    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .header("User-Agent", "XSpring CLI tool")
        .send()
        .await
        .expect("request failed");
    let headers = t.elapsed();

    let status = response.status();
    let content_length = response.content_length();

    // ------------------------------------------------------------------
    // 3. Body download
    // ------------------------------------------------------------------
    let t = Instant::now();
    let body = response.text().await.expect("body read failed");
    let body_read = t.elapsed();

    // ------------------------------------------------------------------
    // 4. Deserialization
    // ------------------------------------------------------------------
    let t = Instant::now();
    let metadata: InitializrMetadata = serde_json::from_str(&body).expect("bad json");
    let deserialize = t.elapsed();

    let total = program_start.elapsed();

    println!("status                : {status}");
    println!("content-length header : {content_length:?}");
    println!("body bytes            : {}", body.len());
    println!("dependency categories : {}", metadata.dependencies.values.len());
    println!();
    println!("client build          : {client_build:>10.2?}");
    println!("request -> headers    : {headers:>10.2?}   (dns+tcp+tls+server)");
    println!("body download         : {body_read:>10.2?}");
    println!("deserialize           : {deserialize:>10.2?}");
    println!("------------------------------------");
    println!("total in-process      : {total:>10.2?}");
    println!();

    // ------------------------------------------------------------------
    // 5. Warm connection: a second request on the SAME client reuses the
    //    TCP+TLS connection, so the difference shows what the handshake
    //    and connection setup cost.
    // ------------------------------------------------------------------
    let t = Instant::now();
    let _ = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .expect("second request failed")
        .text()
        .await
        .expect("second body failed");
    println!("2nd request, same client (connection reused): {:?}", t.elapsed());
    println!("  -> the delta vs 'request -> headers' above is the connection setup cost");
    println!();

    // ------------------------------------------------------------------
    // 6. What the CLI itself does, end to end, with and without the cache.
    // ------------------------------------------------------------------
    println!("Now run the real CLI to see the cache in action:");
    println!("  cargo run --release -- --vv list --java   # first run: fills the cache");
    println!("  cargo run --release -- --vv list --java   # second run: served from the cache");
}
