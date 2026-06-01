//! AppImage catalog — the default browse source.
//!
//! Pulls the community AppImageHub feed (https://appimage.github.io/feed.json),
//! which lists ~1400 apps with their GitHub repo. Installing resolves the repo's
//! latest GitHub release to a downloadable `.AppImage` asset.

use std::process::Command;
use std::path::PathBuf;
use xpm_core::error::{Error, Result};

pub const FEED_URL: &str = "https://appimage.github.io/feed.json";
pub const ICON_BASE: &str = "https://appimage.github.io/database/";

/// A browsable catalog app (only entries with a resolvable GitHub repo).
#[derive(Debug, Clone)]
pub struct CatalogEntry {
    pub name: String,
    pub description: String,
    pub categories: Vec<String>,
    /// GitHub "owner/repo" — used to resolve the download.
    pub github: String,
    pub icon_url: Option<String>,
}

fn which(cmd: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|d| d.join(cmd)).find(|p| p.is_file())
}

fn curl_text(url: &str, accept: Option<&str>) -> Result<String> {
    if which("curl").is_none() {
        return Err(Error::Other("curl is required".to_string()));
    }
    let mut cmd = Command::new("curl");
    cmd.args(["-fsSL", "--max-time", "30"]);
    if let Some(a) = accept {
        cmd.args(["-H", &format!("Accept: {}", a)]);
    }
    cmd.args(["-H", "User-Agent: xPackageManager"]);
    cmd.arg(url);
    let out = cmd.output().map_err(|e| Error::NetworkError(e.to_string()))?;
    if !out.status.success() {
        return Err(Error::NetworkError(format!("Request failed: {}", url)));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Fetch and parse the catalog. Sorted by name; only GitHub-backed apps kept.
///
/// The feed is messy — keys may be present-but-null and arrays may contain null
/// elements — so we traverse a `Value` tolerantly rather than via typed structs.
pub fn fetch() -> Result<Vec<CatalogEntry>> {
    Ok(fetch_sources(&[FEED_URL.to_string()]))
}

/// Fetch and merge multiple feed-JSON sources. Per-source errors are logged and
/// skipped; entries are de-duplicated by GitHub repo (case-insensitive), then name.
pub fn fetch_sources(urls: &[String]) -> Vec<CatalogEntry> {
    let mut merged: Vec<CatalogEntry> = Vec::new();
    let mut seen_repo = std::collections::HashSet::new();
    let mut seen_name = std::collections::HashSet::new();

    for url in urls {
        if url.trim().is_empty() {
            continue;
        }
        match fetch_one(url) {
            Ok(entries) => {
                for e in entries {
                    let repo_key = e.github.to_lowercase();
                    let name_key = e.name.to_lowercase();
                    if !seen_repo.insert(repo_key) || !seen_name.insert(name_key) {
                        continue;
                    }
                    merged.push(e);
                }
            }
            Err(err) => tracing::warn!("AppImage source failed ({}): {}", url, err),
        }
    }
    merged.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    merged
}

/// Fetch and parse a single feed-JSON source.
pub fn fetch_one(url: &str) -> Result<Vec<CatalogEntry>> {
    let body = curl_text(url, Some("application/json"))?;
    let root: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| Error::Other(format!("Bad feed JSON: {}", e)))?;

    let items = root
        .get("items")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::Other("Feed has no items".to_string()))?;

    let mut entries: Vec<CatalogEntry> = Vec::new();
    for item in items {
        let name = match item.get("name").and_then(|v| v.as_str()) {
            Some(n) if !n.is_empty() => n.to_string(),
            _ => continue,
        };

        // First GitHub "owner/repo" link.
        let github = item
            .get("links")
            .and_then(|v| v.as_array())
            .and_then(|links| {
                links.iter().find_map(|l| {
                    if l.get("type").and_then(|t| t.as_str()) == Some("GitHub") {
                        l.get("url").and_then(|u| u.as_str()).filter(|u| u.contains('/')).map(String::from)
                    } else {
                        None
                    }
                })
            });
        let Some(github) = github else { continue };

        let description = item
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let categories: Vec<String> = item
            .get("categories")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|c| c.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let icon_url = item
            .get("icons")
            .and_then(|v| v.as_array())
            .and_then(|a| a.iter().find_map(|i| i.as_str()))
            .map(|i| {
                if i.starts_with("http://") || i.starts_with("https://") {
                    i.to_string()
                } else {
                    format!("{}{}", ICON_BASE, i)
                }
            });

        entries.push(CatalogEntry { name, description, categories, github, icon_url });
    }
    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(entries)
}

/// Resolve a GitHub "owner/repo" to a direct `.AppImage` download URL from its
/// latest release. Prefers an x86_64/amd64 asset.
pub fn resolve_download(github: &str) -> Result<String> {
    let api = format!("https://api.github.com/repos/{}/releases/latest", github.trim_matches('/'));
    let body = curl_text(&api, Some("application/vnd.github+json"))?;
    let release: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| Error::Other(format!("Bad release JSON: {}", e)))?;

    let assets = release
        .get("assets")
        .and_then(|a| a.as_array())
        .ok_or_else(|| Error::Other("No release assets found".to_string()))?;

    let urls: Vec<(String, String)> = assets
        .iter()
        .filter_map(|a| {
            let name = a.get("name")?.as_str()?.to_string();
            let url = a.get("browser_download_url")?.as_str()?.to_string();
            if name.to_lowercase().ends_with(".appimage") {
                Some((name, url))
            } else {
                None
            }
        })
        .collect();

    if urls.is_empty() {
        return Err(Error::Other(format!(
            "No .AppImage asset in the latest release of {}",
            github
        )));
    }
    // Prefer 64-bit desktop builds.
    let preferred = urls.iter().find(|(n, _)| {
        let l = n.to_lowercase();
        l.contains("x86_64") || l.contains("amd64") || l.contains("x86-64")
    });
    Ok(preferred.unwrap_or(&urls[0]).1.clone())
}
