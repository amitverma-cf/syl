use std::path::{Path, PathBuf};

use crate::PluginRegistryError;

fn cache_file_name(url: &str) -> &str {
    url.rsplit('/').next().unwrap_or("download")
}

pub fn is_cached(url: &str, cache_dir: &Path) -> bool {
    cache_dir.join(cache_file_name(url)).exists()
}

pub fn download_to_cache(url: &str, cache_dir: &Path) -> Result<PathBuf, PluginRegistryError> {
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

    Ok(dest)
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
