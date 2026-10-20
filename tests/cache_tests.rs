//! Behavioural tests for the metadata cache.
//!
//! A tiny HTTP server stands in for `start.spring.io` so the whole
//! fetch → cache → revalidate → fallback cycle can be exercised
//! deterministically, without touching the real network.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use reqwest::Client;
use xspring::client::cache::{MetadataCache, DEFAULT_TTL_SECONDS};
use xspring::client::spring_initializr::get_metadata_with;

const METADATA: &str = include_str!("fixtures/metadata.json");

/// A request the fake server saw.
#[derive(Debug, Clone)]
struct SeenRequest {
    /// Value of the `If-None-Match` header, if any.
    if_none_match: Option<String>,
}

/// What the fake server should answer with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Behaviour {
    /// Always answer `200` with the metadata body.
    Fresh,
    /// Answer `304` when the request carries an `If-None-Match` header,
    /// `200` otherwise.
    Revalidate,
    /// Answer `200` with a body that does not match the expected schema.
    Broken,
}

struct FakeServer {
    url: String,
    requests: Arc<AtomicUsize>,
    seen: Arc<Mutex<Vec<SeenRequest>>>,
}

impl FakeServer {
    fn start(behaviour: Behaviour) -> FakeServer {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();

        let requests = Arc::new(AtomicUsize::new(0));
        let seen = Arc::new(Mutex::new(Vec::new()));

        let thread_requests = Arc::clone(&requests);
        let thread_seen = Arc::clone(&seen);

        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };

                let request = read_request(&stream);
                thread_requests.fetch_add(1, Ordering::SeqCst);
                thread_seen.lock().expect("lock").push(request);

                let conditional = thread_seen
                    .lock()
                    .expect("lock")
                    .last()
                    .and_then(|request| request.if_none_match.clone())
                    .is_some();

                let mut stream = stream;
                match behaviour {
                    Behaviour::Fresh => {
                        write_response(&mut stream, "200 OK", METADATA, Some("\"tag-1\""))
                    }
                    Behaviour::Broken => write_response(
                        &mut stream,
                        "200 OK",
                        "{\"dependencies\":[],\"unexpected\":true}",
                        Some("\"tag-2\""),
                    ),
                    Behaviour::Revalidate if conditional => {
                        write_response(&mut stream, "304 Not Modified", "", Some("\"tag-1\""))
                    }
                    Behaviour::Revalidate => {
                        write_response(&mut stream, "200 OK", METADATA, Some("\"tag-1\""))
                    }
                }
            }
        });

        FakeServer {
            url: format!("http://127.0.0.1:{port}/"),
            requests,
            seen,
        }
    }

    fn request_count(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }

    fn last_request(&self) -> Option<SeenRequest> {
        self.seen.lock().expect("lock").last().cloned()
    }
}

fn read_request(stream: &TcpStream) -> SeenRequest {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    let _ = reader.read_line(&mut line);

    let mut if_none_match = None;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).unwrap_or(0) == 0 {
            break;
        }
        if header.trim().is_empty() {
            break;
        }
        if let Some(value) = header.to_ascii_lowercase().strip_prefix("if-none-match:") {
            if_none_match = Some(value.trim().to_string());
        }
    }

    SeenRequest { if_none_match }
}

fn write_response(stream: &mut TcpStream, status: &str, body: &str, etag: Option<&str>) {
    let mut response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n",
        body.len()
    );
    if let Some(etag) = etag {
        response.push_str(&format!("ETag: {etag}\r\n"));
    }
    response.push_str("Connection: close\r\n\r\n");
    response.push_str(body);

    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

/// A port nothing is listening on.
fn dead_url() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    format!("http://127.0.0.1:{port}/")
}

fn client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("client")
}

fn cache_path(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("xspring-cache-tests");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{name}.json"));
    let _ = std::fs::remove_file(&path);
    path
}

#[tokio::test]
async fn fresh_cache_is_served_without_any_network_request() {
    let server = FakeServer::start(Behaviour::Fresh);
    let path = cache_path("fresh");
    let client = client();

    // First call populates the cache.
    let cache = MetadataCache::open(path.clone(), true);
    let metadata = get_metadata_with(cache, &server.url, DEFAULT_TTL_SECONDS, &client)
        .await
        .expect("first fetch");
    assert!(!metadata.dependencies.values.is_empty());
    assert_eq!(server.request_count(), 1, "the first call hits the network");
    assert!(path.exists(), "the cache file is written");

    // Second call is served from the cache.
    let cache = MetadataCache::open(path.clone(), true);
    let metadata = get_metadata_with(cache, &server.url, DEFAULT_TTL_SECONDS, &client)
        .await
        .expect("second fetch");
    assert!(!metadata.dependencies.values.is_empty());
    assert_eq!(
        server.request_count(),
        1,
        "a fresh cache must not trigger a second request"
    );
}

#[tokio::test]
async fn stale_cache_is_revalidated_with_if_none_match() {
    let server = FakeServer::start(Behaviour::Revalidate);
    let path = cache_path("stale");
    let client = client();

    let cache = MetadataCache::open(path.clone(), true);
    get_metadata_with(cache, &server.url, DEFAULT_TTL_SECONDS, &client)
        .await
        .expect("first fetch");

    // A TTL of zero makes every entry stale, so the next call must go to the
    // server with a conditional request.
    let cache = MetadataCache::open(path.clone(), true);
    let metadata = get_metadata_with(cache, &server.url, 0, &client)
        .await
        .expect("revalidation");
    assert!(!metadata.dependencies.values.is_empty());

    assert_eq!(server.request_count(), 2, "a stale cache is revalidated");
    let request = server.last_request().expect("a request was seen");
    assert!(
        request.if_none_match.is_some(),
        "revalidation must send If-None-Match, got: {request:?}"
    );
}

#[tokio::test]
async fn unreachable_server_falls_back_to_the_cached_copy() {
    let path = cache_path("fallback");

    // Seed the cache with a document the dead server never sees.
    let mut cache = MetadataCache::open(path.clone(), true);
    cache
        .store(
            METADATA.to_string(),
            Some("\"tag-1\"".to_string()),
            xspring::util::now_secs(),
        )
        .expect("seed the cache");

    let client = client();
    let cache = MetadataCache::open(path.clone(), true);
    let metadata = get_metadata_with(cache, &dead_url(), 0, &client)
        .await
        .expect("the cached copy must be used when the network is down");
    assert!(!metadata.dependencies.values.is_empty());
}

#[tokio::test]
async fn a_slow_server_hits_the_request_timeout() {
    // A server that accepts the connection but never answers.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    std::thread::spawn(move || {
        let _stream = listener.accept();
        std::thread::sleep(Duration::from_secs(30));
    });

    let client = Client::builder()
        .timeout(Duration::from_millis(300))
        .build()
        .expect("client");

    let path = cache_path("timeout");
    let cache = MetadataCache::open(path, true);
    let err = get_metadata_with(cache, &format!("http://127.0.0.1:{port}/"), 0, &client)
        .await
        .expect_err("a hung server must not hang the CLI forever");
    let message = format!("{err:#}").to_lowercase();
    assert!(
        message.contains("timed out") || message.contains("timeout"),
        "expected a timeout error, got: {err:#}"
    );
}

#[tokio::test]
async fn a_corrupt_cache_file_is_discarded() {
    let path = cache_path("corrupt");
    std::fs::write(&path, b"{ not json").expect("write");

    let cache = MetadataCache::open(path.clone(), true);
    assert!(cache.entry().is_none(), "a corrupt cache must not be used");
    assert!(!path.exists(), "a corrupt cache file is removed");
}

#[tokio::test]
async fn an_unparsable_response_does_not_poison_the_cache() {
    // The server answers with a document that does not match the schema
    // (upstream drift). The cached document that *does* parse has to survive,
    // otherwise every later run fails until the file is deleted by hand.
    let server = FakeServer::start(Behaviour::Broken);
    let path = cache_path("poison");
    let client = client();

    // Seed the cache with a good document.
    let mut cache = MetadataCache::open(path.clone(), true);
    cache
        .store(
            METADATA.to_string(),
            Some("\"tag-1\"".to_string()),
            xspring::util::now_secs(),
        )
        .expect("seed the cache");

    // The broken response must not be adopted...
    let cache = MetadataCache::open(path.clone(), true);
    let metadata = get_metadata_with(cache, &server.url, 0, &client)
        .await
        .expect("the cached copy is used when the new document cannot be parsed");
    assert!(!metadata.dependencies.values.is_empty());

    // ...and the good cached document must still be on disk.
    let cache = MetadataCache::open(path.clone(), true);
    let entry = cache.entry().expect("the cached entry survived");
    assert_eq!(
        entry.body, METADATA,
        "the unparsable response must not replace the cached body"
    );
    assert_eq!(
        entry.etag.as_deref(),
        Some("\"tag-1\""),
        "the cached validator must not be replaced either"
    );
}
