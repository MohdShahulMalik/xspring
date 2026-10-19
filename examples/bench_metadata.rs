use std::time::Instant;
use xspring::client::spring_initializr::get_metadata;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // 1. The path the CLI actually uses today
    for i in 1..=3 {
        let t = Instant::now();
        let _ = get_metadata().await;
        println!("get_metadata() run {i}: {:?}", t.elapsed());
    }

    // 2. Reused client (connection pooling) — same URL
    let client = reqwest::Client::new();
    for i in 1..=3 {
        let t = Instant::now();
        let _ = client
            .get("https://start.spring.io")
            .header("Accept", "application/json")
            .header("User-Agent", "XSpring CLI tool")
            .send()
            .await
            .unwrap();
        println!("reused-client run {i}: {:?}", t.elapsed());
    }

    // 3. Byte size of the payload (deserialization cost reference)
    let body = client
        .get("https://start.spring.io")
        .header("Accept", "application/json")
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    println!("payload size: {} bytes", body.len());

    let t = Instant::now();
    let _md: xspring::models::spring::InitializrMetadata = serde_json::from_str(&body).unwrap();
    println!("deserialize only: {:?}", t.elapsed());
}
