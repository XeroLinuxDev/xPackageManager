use crate::elf;
use crate::integration;
use crate::manifest::{self, AppImageEntry};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{info, warn};
use xpm_core::{
    error::{Error, Result},
    operation::{Operation, OperationKind, OperationResult},
    package::{Package, PackageBackend, PackageInfo, PackageStatus, SearchResult, UpdateInfo, Version},
    source::{PackageSource, ProgressCallback},
};

/// Log sink for an in-progress operation. Lines are streamed to the UI terminal.
/// Called synchronously on the operation's own thread, so no Send/Sync bound.
pub type LogFn<'a> = dyn Fn(&str) + 'a;

pub struct AppImageBackend {
    /// Where new AppImages are downloaded/copied to. Defaults to the store dir.
    /// The manifest, icons and desktop entries always live in standard XDG dirs,
    /// and each manifest entry records an absolute path, so changing this is safe
    /// and existing installs keep working wherever they live.
    install_dir: std::path::PathBuf,
}

impl AppImageBackend {
    pub fn new() -> Result<Self> {
        Self::with_dir(None)
    }

    /// Construct with a custom install directory (None => default store dir).
    pub fn with_dir(dir: Option<std::path::PathBuf>) -> Result<Self> {
        let install_dir = dir
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(manifest::store_dir);
        // Best-effort: make sure the dirs exist so listing/installing never errors.
        let _ = std::fs::create_dir_all(manifest::store_dir());
        let _ = std::fs::create_dir_all(&install_dir);
        Ok(Self { install_dir })
    }

    /// Synchronous manifest read for callers that aren't on an async runtime.
    pub fn list_entries(&self) -> Vec<AppImageEntry> {
        manifest::load()
    }

    fn is_url(source: &str) -> bool {
        source.starts_with("http://") || source.starts_with("https://")
    }

    /// Install from a local path or an http(s) URL. Streams progress via `log`.
    pub fn install(&self, source: &str, log: &LogFn<'_>) -> Result<AppImageEntry> {
        let store = self.install_dir.clone();
        std::fs::create_dir_all(&store)?;

        // Resolve the source down to a local file we own inside the store dir.
        let (staged, source_url): (PathBuf, Option<String>) = if Self::is_url(source) {
            let file_name = source
                .rsplit('/')
                .next()
                .filter(|s| !s.is_empty())
                .unwrap_or("download.AppImage");
            let dest = store.join(manifest::sanitize_name(file_name)).with_extension("AppImage");
            log(&format!("Downloading {}\n", source));
            download(source, &dest, log)?;
            (dest, Some(source.to_string()))
        } else {
            let src = Path::new(source);
            if !src.exists() {
                return Err(Error::Other(format!("File not found: {}", source)));
            }
            let file_name = src.file_name().and_then(|f| f.to_str()).unwrap_or("app.AppImage");
            let dest = store.join(file_name);
            log(&format!("Copying {}\n", source));
            std::fs::copy(src, &dest)?;
            (dest, Some(source.to_string()))
        };

        // Make executable (0o755).
        chmod_exec(&staged)?;

        let raw_name = staged.file_name().and_then(|f| f.to_str()).unwrap_or("appimage");
        let name = manifest::sanitize_name(raw_name);

        // Detect embedded update info before touching the menu.
        log("Checking update support…\n");
        let update_info = elf::read_upd_info(&staged);
        let supports_update = update_info.is_some();
        if supports_update {
            log("Update support: yes (embedded update info found)\n");
        } else {
            log("Update support: no (no embedded update info)\n");
        }

        // Desktop/menu integration.
        log("Integrating into application menu…\n");
        let integ = integration::integrate(&staged, &name, raw_name.trim_end_matches(".AppImage"));

        let size = std::fs::metadata(&staged).map(|m| m.len()).unwrap_or(0);
        let entry = AppImageEntry {
            name: name.clone(),
            display_name: integ.display_name,
            version: integ.version.unwrap_or_else(|| "unknown".to_string()),
            path: staged.to_string_lossy().to_string(),
            source_url,
            github: None,
            update_info,
            supports_update,
            icon_path: integ.icon_path,
            desktop_path: integ.desktop_path,
            size,
        };

        // Upsert into the manifest (replace any existing entry with the same id).
        let mut entries = manifest::load();
        entries.retain(|e| e.name != entry.name);
        entries.push(entry.clone());
        manifest::save(&entries).map_err(Error::IoError)?;

        if !fuse_available() {
            log("Note: FUSE not detected. Running AppImages needs 'fuse2' (or 'fuse3'); install it if the app won't launch.\n");
        }
        log(&format!("Installed {}\n", entry.display_name));
        Ok(entry)
    }

    /// Resolve a catalog GitHub repo to its latest .AppImage and install it.
    pub fn install_from_github(&self, github: &str, log: &LogFn<'_>) -> Result<AppImageEntry> {
        log(&format!("Resolving latest release for {}…\n", github));
        let url = crate::catalog::resolve_download(github)?;
        let mut entry = self.install(&url, log)?;
        // Record the catalog repo so the catalog can show this app as installed.
        entry.github = Some(github.to_string());
        let mut entries = manifest::load();
        entries.retain(|e| e.name != entry.name);
        entries.push(entry.clone());
        manifest::save(&entries).map_err(Error::IoError)?;
        Ok(entry)
    }

    /// Remove a tracked AppImage by id: delete binary, icon, desktop entry, manifest row.
    pub fn remove_app(&self, name: &str, log: &LogFn<'_>) -> Result<()> {
        let mut entries = manifest::load();
        let Some(pos) = entries.iter().position(|e| e.name == name) else {
            return Err(Error::PackageNotFound(name.to_string()));
        };
        let entry = entries.remove(pos);

        log(&format!("Removing {}\n", entry.display_name));
        if let Err(e) = std::fs::remove_file(&entry.path) {
            warn!("appimage: failed to remove binary {}: {}", entry.path, e);
        }
        integration::deintegrate(&entry.icon_path, &entry.desktop_path);
        manifest::save(&entries).map_err(Error::IoError)?;
        log("Removed.\n");
        Ok(())
    }

    /// Update a tracked AppImage. Requires embedded update info.
    /// Uses `appimageupdatetool` when available, else re-downloads the source URL.
    pub fn update_app(&self, name: &str, log: &LogFn<'_>) -> Result<AppImageEntry> {
        let entries = manifest::load();
        let Some(entry) = entries.iter().find(|e| e.name == name).cloned() else {
            return Err(Error::PackageNotFound(name.to_string()));
        };
        if !entry.supports_update {
            return Err(Error::Other(format!(
                "{} does not support updating (no embedded update info)",
                entry.display_name
            )));
        }

        let path = PathBuf::from(&entry.path);
        if which("appimageupdatetool").is_some() {
            log("Updating via appimageupdatetool…\n");
            let status = Command::new("appimageupdatetool")
                .arg("--overwrite")
                .arg(&path)
                .status()
                .map_err(|e| Error::Other(format!("appimageupdatetool failed: {}", e)))?;
            if !status.success() {
                return Err(Error::Other("Update failed".to_string()));
            }
            chmod_exec(&path)?;
        } else if let Some(url) = &entry.source_url {
            if !Self::is_url(url) {
                return Err(Error::Other(
                    "No update tool and original source is a local file".to_string(),
                ));
            }
            log("appimageupdatetool not found — re-downloading from source URL…\n");
            download(url, &path, log)?;
            chmod_exec(&path)?;
        } else {
            return Err(Error::Other(
                "No update tool (appimageupdatetool) and no source URL recorded".to_string(),
            ));
        }

        // Re-integrate in case metadata/icon changed, and refresh manifest row.
        let integ = integration::integrate(&path, &entry.name, &entry.display_name);
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(entry.size);
        let updated = AppImageEntry {
            display_name: integ.display_name,
            version: integ.version.unwrap_or(entry.version.clone()),
            update_info: elf::read_upd_info(&path),
            supports_update: true,
            icon_path: integ.icon_path.or(entry.icon_path.clone()),
            desktop_path: integ.desktop_path.or(entry.desktop_path.clone()),
            size,
            ..entry
        };
        let mut entries = manifest::load();
        entries.retain(|e| e.name != updated.name);
        entries.push(updated.clone());
        manifest::save(&entries).map_err(Error::IoError)?;
        log(&format!("Updated {}\n", updated.display_name));
        Ok(updated)
    }

    /// Re-fetch the app from its recorded source (GitHub latest, URL, or local
    /// file) and replace it in place. Used for AppImages that lack embedded
    /// update info — the only way to "update" them is to reinstall.
    pub fn reinstall_app(&self, name: &str, log: &LogFn<'_>) -> Result<AppImageEntry> {
        let entries = manifest::load();
        let Some(entry) = entries.iter().find(|e| e.name == name).cloned() else {
            return Err(Error::PackageNotFound(name.to_string()));
        };
        let path = PathBuf::from(&entry.path);
        log(&format!("Reinstalling {}…\n", entry.display_name));

        // Fetch a fresh binary into the same path.
        if let Some(gh) = &entry.github {
            log(&format!("Resolving latest release for {}…\n", gh));
            let url = crate::catalog::resolve_download(gh)?;
            download(&url, &path, log)?;
        } else if let Some(src) = &entry.source_url {
            if Self::is_url(src) {
                log("Re-downloading from source URL…\n");
                download(src, &path, log)?;
            } else {
                let p = Path::new(src);
                if !p.exists() {
                    return Err(Error::Other(format!(
                        "Original file no longer available: {}",
                        src
                    )));
                }
                log(&format!("Re-copying from {}…\n", src));
                std::fs::copy(p, &path)?;
            }
        } else {
            return Err(Error::Other("No source recorded to reinstall from".to_string()));
        }
        chmod_exec(&path)?;

        let update_info = elf::read_upd_info(&path);
        let supports_update = update_info.is_some();
        let integ = integration::integrate(&path, &entry.name, &entry.display_name);
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(entry.size);
        let updated = AppImageEntry {
            display_name: integ.display_name,
            version: integ.version.unwrap_or(entry.version.clone()),
            update_info,
            supports_update,
            icon_path: integ.icon_path.or(entry.icon_path.clone()),
            desktop_path: integ.desktop_path.or(entry.desktop_path.clone()),
            size,
            ..entry
        };
        let mut entries = manifest::load();
        entries.retain(|e| e.name != updated.name);
        entries.push(updated.clone());
        manifest::save(&entries).map_err(Error::IoError)?;
        log(&format!("Reinstalled {}\n", updated.display_name));
        Ok(updated)
    }

    fn entry_to_package(entry: &AppImageEntry) -> Package {
        Package::new(
            entry.name.clone(),
            Version::new(&entry.version),
            entry.display_name.clone(),
            PackageBackend::AppImage,
            PackageStatus::Installed,
            "appimage",
        )
    }
}

#[async_trait]
impl PackageSource for AppImageBackend {
    fn source_id(&self) -> &str {
        "appimage"
    }

    fn display_name(&self) -> &str {
        "AppImage"
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn search(&self, _query: &str) -> Result<Vec<SearchResult>> {
        // No catalog — AppImages are installed from a file or URL.
        Ok(Vec::new())
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        tokio::task::spawn_blocking(|| {
            manifest::load().iter().map(AppImageBackend::entry_to_package).collect()
        })
        .await
        .map_err(|e| Error::Other(e.to_string()))
    }

    async fn list_updates(&self) -> Result<Vec<UpdateInfo>> {
        // Capability is shown per-app in the UI; checking real updates needs
        // network per file, so it stays user-initiated. No startup probing.
        Ok(Vec::new())
    }

    async fn get_package_info(&self, name: &str) -> Result<PackageInfo> {
        let name = name.to_string();
        tokio::task::spawn_blocking(move || {
            let entries = manifest::load();
            let entry = entries
                .iter()
                .find(|e| e.name == name)
                .ok_or_else(|| Error::PackageNotFound(name.clone()))?;
            Ok(PackageInfo {
                package: AppImageBackend::entry_to_package(entry),
                url: entry.source_url.clone(),
                licenses: Vec::new(),
                groups: Vec::new(),
                depends: Vec::new(),
                optdepends: Vec::new(),
                provides: Vec::new(),
                conflicts: Vec::new(),
                replaces: Vec::new(),
                installed_size: entry.size,
                download_size: 0,
                build_date: None,
                install_date: None,
                packager: None,
                arch: String::new(),
                reason: None,
            })
        })
        .await
        .map_err(|e| Error::Other(e.to_string()))?
    }

    async fn execute(&self, operation: Operation) -> Result<OperationResult> {
        self.execute_with_progress(operation, Box::new(|_| {})).await
    }

    async fn execute_with_progress(
        &self,
        operation: Operation,
        _progress: ProgressCallback,
    ) -> Result<OperationResult> {
        let start = std::time::Instant::now();
        info!("AppImage operation: {:?}", operation.kind);
        let target = operation.packages.first().cloned().unwrap_or_default();

        let backend = AppImageBackend::with_dir(Some(self.install_dir.clone()))?;
        let kind = operation.kind.clone();
        let res = tokio::task::spawn_blocking(move || {
            let noop: &LogFn = &|_: &str| {};
            match kind {
                OperationKind::Install => backend.install(&target, noop).map(|_| ()),
                OperationKind::Remove | OperationKind::RemoveWithDeps => {
                    backend.remove_app(&target, noop)
                }
                OperationKind::Update => backend.update_app(&target, noop).map(|_| ()),
                _ => Ok(()),
            }
        })
        .await
        .map_err(|e| Error::Other(e.to_string()))?;

        let dur = start.elapsed().as_millis() as u64;
        Ok(match res {
            Ok(()) => OperationResult::success(operation, Vec::new(), dur),
            Err(e) => OperationResult::failure(operation, e.to_string(), dur),
        })
    }

    async fn sync_databases(&self) -> Result<()> {
        Ok(())
    }

    async fn get_cache_size(&self) -> Result<u64> {
        Ok(0)
    }

    async fn clean_cache(&self, _keep_versions: usize) -> Result<u64> {
        Ok(0)
    }

    async fn list_orphans(&self) -> Result<Vec<Package>> {
        Ok(Vec::new())
    }
}

fn chmod_exec(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

/// True if a FUSE runtime is present (needed to *run* type-2 AppImages).
fn fuse_available() -> bool {
    if which("fusermount").is_some() || which("fusermount3").is_some() {
        return true;
    }
    let libs = [
        "/usr/lib/libfuse.so.2",
        "/usr/lib/libfuse3.so.3",
        "/usr/lib/x86_64-linux-gnu/libfuse.so.2",
        "/usr/lib/x86_64-linux-gnu/libfuse3.so.3",
    ];
    libs.iter().any(|p| Path::new(p).exists())
}

fn which(cmd: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(cmd))
        .find(|p| p.is_file())
}

/// Download `url` to `dest` using curl, streaming curl's stderr progress to `log`.
fn download(url: &str, dest: &Path, log: &LogFn<'_>) -> Result<()> {
    if which("curl").is_none() {
        return Err(Error::Other("curl is required to download AppImages".to_string()));
    }
    let status = Command::new("curl")
        .args(["-fL", "--retry", "2", "-o"])
        .arg(dest)
        .arg(url)
        .status()
        .map_err(|e| Error::NetworkError(e.to_string()))?;
    if !status.success() {
        let _ = std::fs::remove_file(dest);
        return Err(Error::NetworkError(format!("Download failed: {}", url)));
    }
    log("Download complete.\n");
    Ok(())
}
