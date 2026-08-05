//! Verifies that engine/model downloads are actually checked against their declared
//! `sha256` before being trusted — the gap this closes is that engine downloads
//! previously skipped verification entirely (unlike models), even though the registry
//! entry carries a hash meant for exactly this. Uses a tiny real local HTTP server
//! (no mocking) so the full `ureq`-backed download path is genuinely exercised.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::JoinHandle;

use extension_registry::{download_and_extract_zip, download_to_cache, PluginRegistryError};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_dir(label: &str) -> std::path::PathBuf {
    let unique = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "syl-checksum-test-{label}-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Spawns a background thread that accepts exactly one HTTP connection and serves a
/// fixed byte body, then shuts down. Returns the `http://127.0.0.1:<port>/file` URL to
/// request and a handle to join once the test is done with it.
fn serve_once(body: &'static [u8]) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            write_response(&mut stream, body);
        }
    });
    (format!("http://127.0.0.1:{port}/file"), handle)
}

fn write_response(stream: &mut TcpStream, body: &[u8]) {
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

#[test]
fn download_to_cache_accepts_a_download_matching_its_declared_checksum() {
    let body: &'static [u8] = b"trusted engine bytes";
    let expected = sha256_hex(body);
    let (url, handle) = serve_once(body);
    let cache_dir = temp_dir("cache-ok");

    let path = download_to_cache(&url, &cache_dir, Some(&expected)).unwrap();
    assert!(path.exists());
    assert_eq!(std::fs::read(&path).unwrap(), body);

    handle.join().unwrap();
    std::fs::remove_dir_all(&cache_dir).unwrap();
}

#[test]
fn download_to_cache_rejects_and_deletes_a_download_that_fails_its_checksum() {
    let body: &'static [u8] = b"tampered or wrong engine bytes";
    let wrong_hash = "0".repeat(64);
    let (url, handle) = serve_once(body);
    let cache_dir = temp_dir("cache-mismatch");

    let err = download_to_cache(&url, &cache_dir, Some(&wrong_hash)).unwrap_err();
    assert!(matches!(err, PluginRegistryError::ChecksumMismatch { .. }));

    // The whole point: an unverified/tampered artifact must not be left behind for a
    // later caller to pick up and trust.
    let entries: Vec<_> = std::fs::read_dir(&cache_dir).unwrap().collect();
    assert!(
        entries.is_empty(),
        "expected the mismatched download to be cleaned up, found {entries:?}"
    );

    handle.join().unwrap();
    std::fs::remove_dir_all(&cache_dir).unwrap();
}

#[test]
fn download_to_cache_skips_verification_when_no_checksum_is_declared() {
    // Matches existing behavior for registry entries with `"sha256": null` (e.g. local
    // dev overrides) — verification is opt-in per entry, not mandatory.
    let body: &'static [u8] = b"unverified bytes, by design";
    let (url, handle) = serve_once(body);
    let cache_dir = temp_dir("cache-no-checksum");

    let path = download_to_cache(&url, &cache_dir, None).unwrap();
    assert!(path.exists());

    handle.join().unwrap();
    std::fs::remove_dir_all(&cache_dir).unwrap();
}

fn make_zip(entry_name: &str, entry_contents: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        writer
            .start_file::<_, ()>(entry_name, zip::write::FileOptions::default())
            .unwrap();
        writer.write_all(entry_contents).unwrap();
        writer.finish().unwrap();
    }
    buf
}

#[test]
fn download_and_extract_zip_rejects_and_cleans_up_an_archive_that_fails_its_checksum() {
    let zip_bytes = make_zip("lib/engine.dll", b"native code");
    let zip_bytes: &'static [u8] = Box::leak(zip_bytes.into_boxed_slice());
    let wrong_hash = "0".repeat(64);
    let (url, handle) = serve_once(zip_bytes);
    let scratch = temp_dir("zip-mismatch");
    let extract_dir = scratch.join("extracted");

    let err = download_and_extract_zip(&url, &extract_dir, Some(&wrong_hash)).unwrap_err();
    assert!(matches!(err, PluginRegistryError::ChecksumMismatch { .. }));

    // Neither the final extracted directory nor a leftover staging directory should
    // exist — a mismatched zip must never reach the point of being extracted to disk.
    assert!(!extract_dir.exists());
    assert!(!extract_dir.with_extension("download-tmp").exists());

    handle.join().unwrap();
    std::fs::remove_dir_all(&scratch).unwrap();
}

#[test]
fn download_and_extract_zip_extracts_a_real_archive_matching_its_checksum() {
    let zip_bytes = make_zip("lib/engine.dll", b"native code");
    let expected = sha256_hex(&zip_bytes);
    let zip_bytes: &'static [u8] = Box::leak(zip_bytes.into_boxed_slice());
    let (url, handle) = serve_once(zip_bytes);
    let scratch = temp_dir("zip-ok");
    let extract_dir = scratch.join("extracted");

    download_and_extract_zip(&url, &extract_dir, Some(&expected)).unwrap();
    assert!(extract_dir.join("lib").join("engine.dll").exists());

    handle.join().unwrap();
    std::fs::remove_dir_all(&scratch).unwrap();
}
