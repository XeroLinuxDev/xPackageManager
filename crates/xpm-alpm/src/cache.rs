//! Cache management for pacman packages.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use xpm_core::error::{Error, Result};

/// Manages the pacman package cache.
pub struct CacheManager {
    cache_dirs: Vec<PathBuf>,
}

impl CacheManager {
    /// Creates a new cache manager.
    pub fn new(dirs: &[String]) -> Self {
        Self {
            cache_dirs: dirs.iter().map(PathBuf::from).collect(),
        }
    }

    /// Gets the total cache size in bytes.
    pub async fn get_size(&self) -> Result<u64> {
        let mut total = 0u64;

        for dir in &self.cache_dirs {
            if dir.exists() {
                total += Self::dir_size(dir)?;
            }
        }

        Ok(total)
    }

    /// Calculates directory size recursively.
    fn dir_size(path: &Path) -> Result<u64> {
        let mut size = 0u64;

        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let metadata = entry.metadata()?;

                if metadata.is_file() {
                    size += metadata.len();
                } else if metadata.is_dir() {
                    size += Self::dir_size(&entry.path())?;
                }
            }
        }

        Ok(size)
    }

    /// Cleans the cache, keeping only the specified number of versions per package.
    pub async fn clean(&self, keep_versions: usize) -> Result<u64> {
        let mut freed = 0u64;

        for dir in &self.cache_dirs {
            if !dir.exists() {
                continue;
            }

            freed += self.clean_dir(dir, keep_versions)?;
        }

        info!("Cache cleaned, freed {} bytes", freed);
        Ok(freed)
    }

    /// Cleans a single cache directory.
    fn clean_dir(&self, dir: &Path, keep_versions: usize) -> Result<u64> {
        let mut packages: HashMap<String, Vec<(PathBuf, std::time::SystemTime)>> = HashMap::new();
        let mut freed = 0u64;

        // Group package files by package name.
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

            // Parse package name from filename (e.g., "package-1.2.3-1-x86_64.pkg.tar.zst").
            if let Some(pkg_name) = Self::parse_package_name(filename) {
                let mtime = entry.metadata()?.modified().unwrap_or(std::time::UNIX_EPOCH);
                packages.entry(pkg_name).or_default().push((path, mtime));
            }
        }

        // Keep only the newest versions.
        for (name, mut versions) in packages {
            if versions.len() <= keep_versions {
                continue;
            }

            // Sort by modification time (newest first).
            versions.sort_by(|a, b| b.1.cmp(&a.1));

            // Remove older versions.
            for (path, _) in versions.iter().skip(keep_versions) {
                if let Ok(metadata) = fs::metadata(&path) {
                    freed += metadata.len();
                }

                debug!("Removing old package: {:?}", path);
                if let Err(e) = fs::remove_file(&path) {
                    warn!("Failed to remove {:?}: {}", path, e);
                }

                // Also remove signature file if present.
                let sig_path = path.with_extension("sig");
                if sig_path.exists() {
                    fs::remove_file(&sig_path).ok();
                }
            }
        }

        Ok(freed)
    }

    /// Parses package name from a cache filename.
    fn parse_package_name(filename: &str) -> Option<String> {
        // Filename format: name-version-release-arch.pkg.tar.zst
        // We need to extract the name part.

        if !filename.contains(".pkg.tar") {
            return None;
        }

        // Remove extension.
        let base = filename.split(".pkg.tar").next()?;

        // Split by '-' and find where version starts (first digit segment).
        let parts: Vec<&str> = base.split('-').collect();

        if parts.len() < 4 {
            return None;
        }

        // Find the index where version starts (first part that starts with a digit).
        let mut version_idx = parts.len();
        for (i, part) in parts.iter().enumerate() {
            if part.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                version_idx = i;
                break;
            }
        }

        if version_idx == 0 || version_idx >= parts.len() {
            return None;
        }

        Some(parts[..version_idx].join("-"))
    }

    /// Lists all cached packages.
    pub async fn list(&self) -> Result<Vec<CachedPackage>> {
        let mut cached = Vec::new();

        for dir in &self.cache_dirs {
            if !dir.exists() {
                continue;
            }

            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if !path.is_file() {
                    continue;
                }

                let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();

                if filename.contains(".pkg.tar") {
                    let metadata = entry.metadata()?;
                    cached.push(CachedPackage {
                        path,
                        filename,
                        size: metadata.len(),
                    });
                }
            }
        }

        Ok(cached)
    }
}

/// Represents a cached package file.
#[derive(Debug)]
pub struct CachedPackage {
    /// Full path to the cached package.
    pub path: PathBuf,
    /// Filename.
    pub filename: String,
    /// File size in bytes.
    pub size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_package_name() {
        assert_eq!(
            CacheManager::parse_package_name("firefox-120.0-1-x86_64.pkg.tar.zst"),
            Some("firefox".to_string())
        );
        assert_eq!(
            CacheManager::parse_package_name("qt6-base-6.6.1-1-x86_64.pkg.tar.zst"),
            Some("qt6-base".to_string())
        );
        assert_eq!(
            CacheManager::parse_package_name("lib32-mesa-23.3.1-1-x86_64.pkg.tar.zst"),
            Some("lib32-mesa".to_string())
        );
    }
}
