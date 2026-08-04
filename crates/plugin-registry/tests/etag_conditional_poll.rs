//! Verifies the registry poll's conditional-GET (`If-None-Match`/`ETag`) behavior end to
//! end against a real local HTTP server — the server serves the same content forever,
//! keyed by a fixed ETag, so a second poll with that ETag should come back as
//! `304 Not Modified` and a poll with no/mismatched ETag should come back with a body.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::JoinHandle;

use plugin_registry::fetch_remote_registry_conditional;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_dir(label: &str) -> std::path::PathBuf {
    let unique = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "syl-etag-test-{label}-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

const ENGINES_ETAG: &str = "\"engines-etag-1\"";
const MODELS_ETAG: &str = "\"models-etag-1\"";

/// Serves `engines.json`/`models.json` with a fixed ETag each, honoring
/// `If-None-Match` with a real `304`, for exactly `request_count` requests before
/// shutting down.
fn serve_registry(request_count: usize) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = std::thread::spawn(move || {
        for _ in 0..request_count {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            handle_request(&mut stream);
        }
    });
    (format!("http://127.0.0.1:{port}"), handle)
}

fn handle_request(stream: &mut TcpStream) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut request_line = String::new();
    reader.read_line(&mut request_line).unwrap();

    let mut if_none_match: Option<String> = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("If-None-Match: ") {
            if_none_match = Some(value.to_string());
        }
    }

    let (path, etag, body) = if request_line.contains("engines.json") {
        ("engines", ENGINES_ETAG, br#"[]"#.as_slice())
    } else {
        ("models", MODELS_ETAG, br#"[]"#.as_slice())
    };
    let _ = path;

    if if_none_match.as_deref() == Some(etag) {
        let response = "HTTP/1.1 304 Not Modified\r\nConnection: keep-alive\r\n\r\n";
        stream.write_all(response.as_bytes()).unwrap();
    } else {
        let header = format!(
            "HTTP/1.1 200 OK\r\nETag: {etag}\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n",
            body.len()
        );
        stream.write_all(header.as_bytes()).unwrap();
        stream.write_all(body).unwrap();
    }
    stream.flush().unwrap();
}

#[test]
fn first_poll_with_no_prior_etag_returns_the_full_body_and_a_new_etag() {
    let (base_url, handle) = serve_registry(2);

    let result = fetch_remote_registry_conditional(&base_url, None, None).unwrap();

    assert_eq!(result.engines.as_deref(), Some("[]"));
    assert_eq!(result.models.as_deref(), Some("[]"));
    assert_eq!(result.engines_etag.as_deref(), Some(ENGINES_ETAG));
    assert_eq!(result.models_etag.as_deref(), Some(MODELS_ETAG));

    handle.join().unwrap();
}

#[test]
fn a_poll_with_the_current_etag_gets_not_modified_and_no_body() {
    let (base_url, handle) = serve_registry(2);

    let result =
        fetch_remote_registry_conditional(&base_url, Some(ENGINES_ETAG), Some(MODELS_ETAG))
            .unwrap();

    assert_eq!(result.engines, None);
    assert_eq!(result.models, None);
    // The unchanged ETag is still carried forward, so the caller keeps remembering it.
    assert_eq!(result.engines_etag.as_deref(), Some(ENGINES_ETAG));
    assert_eq!(result.models_etag.as_deref(), Some(MODELS_ETAG));

    handle.join().unwrap();
}

#[test]
fn a_stale_etag_still_gets_the_full_body_back() {
    let (base_url, handle) = serve_registry(2);

    let result = fetch_remote_registry_conditional(
        &base_url,
        Some("\"stale-etag\""),
        Some("\"stale-etag\""),
    )
    .unwrap();

    assert_eq!(result.engines.as_deref(), Some("[]"));
    assert_eq!(result.models.as_deref(), Some("[]"));

    handle.join().unwrap();
}

#[test]
fn etag_read_write_round_trips_through_a_real_registry_dir() {
    let dir = temp_dir("read-write");

    assert_eq!(plugin_registry::read_etag(&dir, "engines"), None);

    plugin_registry::write_etag(&dir, "engines", ENGINES_ETAG).unwrap();
    assert_eq!(
        plugin_registry::read_etag(&dir, "engines").as_deref(),
        Some(ENGINES_ETAG)
    );

    // Writing again overwrites, not appends.
    plugin_registry::write_etag(&dir, "engines", "\"newer\"").unwrap();
    assert_eq!(
        plugin_registry::read_etag(&dir, "engines").as_deref(),
        Some("\"newer\"")
    );

    std::fs::remove_dir_all(&dir).unwrap();
}
