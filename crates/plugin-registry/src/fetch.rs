use std::path::{Path, PathBuf};

use crate::PluginRegistryError;

/// Derives a safe cache filename from a download URL. Strips query/fragment, splits on
/// both `/` and `\` (a Windows path separator, which a URL's last segment could still
/// contain), and rejects anything that isn't a single normal path component (no `..`,
/// no empty result, no embedded separator) rather than trusting the URL's tail verbatim —
/// otherwise a crafted `download_url` could smuggle `..` segments into a cache-directory
/// join and write outside the intended cache directory. Falls back to a content hash of
/// the URL when the derived name isn't safe, so a download still succeeds under a stable,
/// collision-resistant name instead of failing outright.
fn cache_file_name(url: &str) -> String {
    let without_fragment = url.split('#').next().unwrap_or(url);
    let without_query = without_fragment
        .split('?')
        .next()
        .unwrap_or(without_fragment);
    let candidate = without_query
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .trim();

    let is_safe = !candidate.is_empty()
        && candidate != "."
        && candidate != ".."
        && !candidate.contains('/')
        && !candidate.contains('\\')
        && !candidate.contains("..")
        && !candidate.contains('\0')
        && !Path::new(candidate).is_absolute();

    if is_safe {
        candidate.to_string()
    } else {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        format!("download-{:x}", hasher.finalize())
    }
}

pub fn is_cached(url: &str, cache_dir: &Path) -> bool {
    cache_dir.join(cache_file_name(url)).exists()
}

pub fn download_to_cache(
    url: &str,
    cache_dir: &Path,
    expected_sha256: Option<&str>,
) -> Result<PathBuf, PluginRegistryError> {
    std::fs::create_dir_all(cache_dir).map_err(|source| PluginRegistryError::Io {
        path: cache_dir.to_path_buf(),
        source,
    })?;

    let dest = cache_dir.join(cache_file_name(url));
    if dest.exists() {
        return Ok(dest);
    }

    let response = ureq::get(url).call().map_err(|err| to_error(url, err))?;
    let mut reader = response.into_reader();
    let mut file = std::fs::File::create(&dest).map_err(|source| PluginRegistryError::Io {
        path: dest.clone(),
        source,
    })?;
    std::io::copy(&mut reader, &mut file).map_err(|source| PluginRegistryError::Io {
        path: dest.clone(),
        source,
    })?;
    drop(file);

    if let Some(expected) = expected_sha256 {
        if let Err(err) = crate::verify_sha256(&dest, expected) {
            let _ = std::fs::remove_file(&dest);
            return Err(err);
        }
    }

    Ok(dest)
}

pub fn download_to_dir(url: &str, dest_dir: &Path) -> Result<PathBuf, PluginRegistryError> {
    std::fs::create_dir_all(dest_dir).map_err(|source| PluginRegistryError::Io {
        path: dest_dir.to_path_buf(),
        source,
    })?;

    let dest = dest_dir.join(cache_file_name(url));
    if dest.exists() {
        return Ok(dest);
    }

    let response = ureq::get(url).call().map_err(|err| to_error(url, err))?;
    let mut reader = response.into_reader();
    let mut file = std::fs::File::create(&dest).map_err(|source| PluginRegistryError::Io {
        path: dest.clone(),
        source,
    })?;
    std::io::copy(&mut reader, &mut file).map_err(|source| PluginRegistryError::Io {
        path: dest.clone(),
        source,
    })?;

    Ok(dest)
}

pub fn download_and_extract_zip(
    url: &str,
    extract_dir: &Path,
    expected_sha256: Option<&str>,
) -> Result<(), PluginRegistryError> {
    if extract_dir.exists() {
        return Ok(());
    }

    let staging_dir = extract_dir.with_extension("download-tmp");
    std::fs::create_dir_all(&staging_dir).map_err(|source| PluginRegistryError::Io {
        path: staging_dir.clone(),
        source,
    })?;
    let zip_path = staging_dir.join(cache_file_name(url));

    let response = ureq::get(url).call().map_err(|err| to_error(url, err))?;
    let mut reader = response.into_reader();
    let mut file = std::fs::File::create(&zip_path).map_err(|source| PluginRegistryError::Io {
        path: zip_path.clone(),
        source,
    })?;
    std::io::copy(&mut reader, &mut file).map_err(|source| PluginRegistryError::Io {
        path: zip_path.clone(),
        source,
    })?;
    drop(file);

    // Verify the downloaded archive itself, before extracting anything from it — a
    // mismatch here means the zip's contents (including whatever native library it
    // holds) are not what the registry entry vouches for, so nothing from it should
    // ever be written out or loaded.
    if let Some(expected) = expected_sha256 {
        if let Err(err) = crate::verify_sha256(&zip_path, expected) {
            let _ = std::fs::remove_dir_all(&staging_dir);
            return Err(err);
        }
    }

    let archive_file =
        std::fs::File::open(&zip_path).map_err(|source| PluginRegistryError::Io {
            path: zip_path.clone(),
            source,
        })?;
    let mut archive = zip::ZipArchive::new(archive_file)
        .map_err(|e| PluginRegistryError::InvalidUrl(format!("{url} is not a valid zip: {e}")))?;
    archive
        .extract(&staging_dir)
        .map_err(|e| PluginRegistryError::InvalidUrl(format!("failed to extract {url}: {e}")))?;

    std::fs::remove_file(&zip_path).ok();
    std::fs::rename(&staging_dir, extract_dir).map_err(|source| PluginRegistryError::Io {
        path: extract_dir.to_path_buf(),
        source,
    })?;

    Ok(())
}

pub fn fetch_remote_registry(base_url: &str) -> Result<(String, String), PluginRegistryError> {
    let engines = fetch_text(&format!("{base_url}/engines.json"))?;
    let models = fetch_text(&format!("{base_url}/models.json"))?;
    Ok((engines, models))
}

fn fetch_text(url: &str) -> Result<String, PluginRegistryError> {
    let response = ureq::get(url).call().map_err(|err| to_error(url, err))?;
    response
        .into_string()
        .map_err(|err| PluginRegistryError::Network {
            url: url.to_string(),
            source: std::io::Error::other(err),
        })
}

fn to_error(url: &str, err: ureq::Error) -> PluginRegistryError {
    match err {
        ureq::Error::Status(status, _) => PluginRegistryError::Http {
            url: url.to_string(),
            status,
        },
        ureq::Error::Transport(transport) => PluginRegistryError::Network {
            url: url.to_string(),
            source: std::io::Error::other(transport.to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::cache_file_name;

    #[test]
    fn keeps_a_plain_filename_as_is() {
        assert_eq!(
            cache_file_name("https://example.com/models/llama.gguf"),
            "llama.gguf"
        );
    }

    #[test]
    fn strips_query_and_fragment() {
        assert_eq!(
            cache_file_name("https://example.com/models/llama.gguf?token=abc#frag"),
            "llama.gguf"
        );
    }

    #[test]
    fn splitting_on_backslash_too_defeats_a_windows_style_traversal_tail() {
        // The original implementation split only on '/', so this whole string (including
        // the embedded "\..\" segments) was treated as one opaque "filename" and joined
        // onto the cache dir as-is — a real escape on Windows. Splitting on '\' too means
        // the traversal segments are discarded the same way forward-slash ones always
        // were, leaving just the final, safe segment.
        let url = r"https://example.com/x\..\..\..\Startup\payload.exe";
        assert_eq!(cache_file_name(url), "payload.exe");
    }

    #[test]
    fn falls_back_to_a_hash_when_the_final_segment_is_itself_a_traversal_marker() {
        let name = cache_file_name("https://example.com/models/..");
        assert!(!name.contains(".."));
        assert!(name.starts_with("download-"));
    }

    #[test]
    fn a_forward_slash_only_path_already_takes_just_the_last_safe_segment() {
        // rsplit('/') alone already discards everything before the final segment, so a
        // pure-forward-slash traversal like this was never actually reachable — the
        // real gap this module closes is backslash-based traversal (see above), since
        // the original implementation split only on '/'.
        assert_eq!(
            cache_file_name("https://example.com/../../../etc/passwd"),
            "passwd"
        );
    }

    #[test]
    fn falls_back_to_a_hash_when_the_tail_is_empty() {
        assert!(cache_file_name("https://example.com/").starts_with("download-"));
    }

    #[test]
    fn is_deterministic_for_the_same_url() {
        let url = "https://example.com/../evil";
        assert_eq!(cache_file_name(url), cache_file_name(url));
    }
}
