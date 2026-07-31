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

pub fn download_and_extract_zip(url: &str, extract_dir: &Path) -> Result<(), PluginRegistryError> {
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
