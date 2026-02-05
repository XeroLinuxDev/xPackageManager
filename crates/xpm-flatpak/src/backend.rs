//! Flatpak backend implementation.

use crate::remote::RemoteManager;
use async_trait::async_trait;
use libflatpak::{gio, prelude::*, Installation, RefKind};
use std::sync::Arc;
use tracing::{debug, info, warn};
use xpm_core::{
    error::{Error, Result},
    operation::{Operation, OperationKind, OperationProgress, OperationResult, OperationStatus},
    package::{Package, PackageBackend, PackageInfo, PackageStatus, SearchResult, UpdateInfo, Version},
    source::{PackageSource, ProgressCallback},
};

/// The Flatpak backend.
pub struct FlatpakBackend {
    remote_manager: RemoteManager,
}

// Flatpak/GLib types are not Send/Sync, so we handle them carefully.
unsafe impl Send for FlatpakBackend {}
unsafe impl Sync for FlatpakBackend {}

impl FlatpakBackend {
    /// Creates a new Flatpak backend.
    pub fn new() -> Result<Self> {
        Ok(Self {
            remote_manager: RemoteManager::new(),
        })
    }

    /// Gets the user installation in a blocking context.
    fn get_user_installation() -> Result<Installation> {
        Installation::new_user(gio::Cancellable::NONE)
            .map_err(|e| Error::BackendUnavailable(format!("User installation: {}", e)))
    }

    /// Gets the system installation in a blocking context.
    fn get_system_installation() -> Result<Installation> {
        Installation::new_system(gio::Cancellable::NONE)
            .map_err(|e| Error::BackendUnavailable(format!("System installation: {}", e)))
    }

    /// Converts an InstalledRef to our Package type.
    fn convert_installed_ref(
        name: &str,
        version: &str,
        description: &str,
        origin: &str,
    ) -> Package {
        Package::new(
            name,
            Version::new(version),
            description,
            PackageBackend::Flatpak,
            PackageStatus::Installed,
            origin,
        )
    }

    /// Lists all available Flatpak apps from configured remotes (e.g., Flathub).
    pub async fn list_available(&self) -> Result<Vec<Package>> {
        tokio::task::spawn_blocking(|| {
            let mut packages = Vec::new();
            let mut seen = std::collections::HashSet::new();

            let installations: Vec<Installation> = [
                Self::get_user_installation().ok(),
                Self::get_system_installation().ok(),
            ]
            .into_iter()
            .flatten()
            .collect();

            // Collect installed app names for status checking
            let mut installed_names = std::collections::HashSet::new();
            for installation in &installations {
                if let Ok(refs) = installation.list_installed_refs(gio::Cancellable::NONE) {
                    for iref in refs {
                        if iref.kind() == RefKind::App {
                            if let Some(name) = iref.name() {
                                installed_names.insert(name.to_string());
                            }
                        }
                    }
                }
            }

            for installation in &installations {
                let remotes = match installation.list_remotes(gio::Cancellable::NONE) {
                    Ok(r) => r,
                    Err(_) => continue,
                };

                for remote in remotes {
                    let remote_name = match remote.name() {
                        Some(n) => n.to_string(),
                        None => continue,
                    };

                    // Get remote refs
                    let refs = match installation.list_remote_refs_sync(&remote_name, gio::Cancellable::NONE) {
                        Ok(r) => r,
                        Err(e) => {
                            warn!("Failed to list refs from {}: {}", remote_name, e);
                            continue;
                        }
                    };

                    for rref in refs {
                        // Only show apps, not runtimes
                        if rref.kind() != RefKind::App {
                            continue;
                        }

                        let name = match rref.name() {
                            Some(n) => n.to_string(),
                            None => continue,
                        };

                        // Skip duplicates
                        if seen.contains(&name) {
                            continue;
                        }
                        seen.insert(name.clone());

                        let branch = rref.branch().map(|s| s.to_string()).unwrap_or_else(|| "stable".to_string());
                        let is_installed = installed_names.contains(&name);

                        // Extract display name from app ID (e.g., org.gimp.GIMP -> GIMP)
                        let display_name = name.split('.').last().unwrap_or(&name).to_string();

                        let status = if is_installed {
                            PackageStatus::Installed
                        } else {
                            PackageStatus::Available
                        };

                        packages.push(Package::new(
                            name,
                            Version::new(&branch),
                            display_name,
                            PackageBackend::Flatpak,
                            status,
                            remote_name.clone(),
                        ));
                    }
                }
            }

            // Sort by display name (stored in description field)
            packages.sort_by(|a, b| a.description.to_lowercase().cmp(&b.description.to_lowercase()));

            Ok(packages)
        })
        .await
        .map_err(|e| Error::Other(e.to_string()))?
    }
}

#[async_trait]
impl PackageSource for FlatpakBackend {
    fn source_id(&self) -> &str {
        "flatpak"
    }

    fn display_name(&self) -> &str {
        "Flatpak"
    }

    async fn is_available(&self) -> bool {
        tokio::task::spawn_blocking(|| {
            Self::get_user_installation().is_ok() || Self::get_system_installation().is_ok()
        })
        .await
        .unwrap_or(false)
    }

    async fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        let query = query.to_string();

        tokio::task::spawn_blocking(move || {
            let mut results = Vec::new();
            let query_lower = query.to_lowercase();

            // Try both user and system installations.
            let installations: Vec<Installation> = [
                Self::get_user_installation().ok(),
                Self::get_system_installation().ok(),
            ]
            .into_iter()
            .flatten()
            .collect();

            for installation in installations {
                let remotes = match installation.list_remotes(gio::Cancellable::NONE) {
                    Ok(r) => r,
                    Err(_) => continue,
                };

                for remote in remotes {
                    let remote_name = match remote.name() {
                        Some(n) => n.to_string(),
                        None => continue,
                    };

                    // Get remote refs.
                    let refs = match installation
                        .list_remote_refs_sync(&remote_name, gio::Cancellable::NONE)
                    {
                        Ok(r) => r,
                        Err(_) => continue,
                    };

                    for rref in refs {
                        // Only show apps, not runtimes.
                        if rref.kind() != RefKind::App {
                            continue;
                        }

                        let name = match rref.name() {
                            Some(n) => n.to_string(),
                            None => continue,
                        };

                        if !name.to_lowercase().contains(&query_lower) {
                            continue;
                        }

                        let arch = rref.arch().map(|s| s.to_string()).unwrap_or_default();
                        let branch = rref.branch().map(|s| s.to_string()).unwrap_or_default();

                        // Check if installed.
                        let installed = installation
                            .installed_ref(RefKind::App, &name, Some(&arch), Some(&branch), gio::Cancellable::NONE)
                            .is_ok();

                        results.push(SearchResult {
                            name: name.clone(),
                            version: Version::new(&branch),
                            description: name.clone(), // Remote refs don't have descriptions.
                            backend: PackageBackend::Flatpak,
                            repository: remote_name.clone(),
                            installed,
                            installed_version: None,
                        });
                    }
                }
            }

            // Deduplicate results.
            results.sort_by(|a, b| a.name.cmp(&b.name));
            results.dedup_by(|a, b| a.name == b.name);

            Ok(results)
        })
        .await
        .map_err(|e| Error::Other(e.to_string()))?
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        tokio::task::spawn_blocking(|| {
            let mut packages = Vec::new();

            let installations: Vec<Installation> = [
                Self::get_user_installation().ok(),
                Self::get_system_installation().ok(),
            ]
            .into_iter()
            .flatten()
            .collect();

            for installation in installations {
                let refs = match installation.list_installed_refs(gio::Cancellable::NONE) {
                    Ok(r) => r,
                    Err(_) => continue,
                };

                for iref in refs {
                    // Only show apps, not runtimes.
                    if iref.kind() != RefKind::App {
                        continue;
                    }

                    let name = iref.name().map(|s| s.to_string()).unwrap_or_default();
                    let version = iref
                        .appdata_version()
                        .or_else(|| iref.branch())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    let description = iref
                        .appdata_name()
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    let origin = iref.origin().map(|s| s.to_string()).unwrap_or_default();

                    packages.push(Package::new(
                        name,
                        Version::new(&version),
                        description,
                        PackageBackend::Flatpak,
                        PackageStatus::Installed,
                        origin,
                    ));
                }
            }

            Ok(packages)
        })
        .await
        .map_err(|e| Error::Other(e.to_string()))?
    }

    async fn list_updates(&self) -> Result<Vec<UpdateInfo>> {
        tokio::task::spawn_blocking(|| {
            let mut updates = Vec::new();

            let installations: Vec<Installation> = [
                Self::get_user_installation().ok(),
                Self::get_system_installation().ok(),
            ]
            .into_iter()
            .flatten()
            .collect();

            for installation in installations {
                let refs = match installation
                    .list_installed_refs_for_update(gio::Cancellable::NONE)
                {
                    Ok(r) => r,
                    Err(_) => continue,
                };

                for iref in refs {
                    if iref.kind() != RefKind::App {
                        continue;
                    }

                    let name = iref.name().map(|s| s.to_string()).unwrap_or_default();
                    let current = iref
                        .appdata_version()
                        .or_else(|| iref.branch())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    let origin = iref.origin().map(|s| s.to_string()).unwrap_or_default();

                    updates.push(UpdateInfo {
                        name,
                        current_version: Version::new(&current),
                        new_version: Version::new(&current), // Flatpak doesn't expose new version easily.
                        backend: PackageBackend::Flatpak,
                        repository: origin,
                        download_size: iref.installed_size() as u64,
                    });
                }
            }

            Ok(updates)
        })
        .await
        .map_err(|e| Error::Other(e.to_string()))?
    }

    async fn get_package_info(&self, name: &str) -> Result<PackageInfo> {
        let name = name.to_string();

        tokio::task::spawn_blocking(move || {
            let installations: Vec<Installation> = [
                Self::get_user_installation().ok(),
                Self::get_system_installation().ok(),
            ]
            .into_iter()
            .flatten()
            .collect();

            for installation in installations {
                let refs = match installation.list_installed_refs(gio::Cancellable::NONE) {
                    Ok(r) => r,
                    Err(_) => continue,
                };

                for iref in refs {
                    let ref_name = iref.name().map(|s| s.to_string()).unwrap_or_default();
                    if ref_name == name {
                        let version = iref
                            .appdata_version()
                            .or_else(|| iref.branch())
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| "unknown".to_string());
                        let description = iref
                            .appdata_name()
                            .map(|s| s.to_string())
                            .unwrap_or_default();
                        let origin = iref.origin().map(|s| s.to_string()).unwrap_or_default();

                        let package = Package::new(
                            ref_name,
                            Version::new(&version),
                            description,
                            PackageBackend::Flatpak,
                            PackageStatus::Installed,
                            origin,
                        );

                        return Ok(PackageInfo {
                            package,
                            url: None,
                            licenses: Vec::new(),
                            groups: Vec::new(),
                            depends: Vec::new(),
                            optdepends: Vec::new(),
                            provides: Vec::new(),
                            conflicts: Vec::new(),
                            replaces: Vec::new(),
                            installed_size: iref.installed_size() as u64,
                            download_size: 0,
                            build_date: None,
                            install_date: None,
                            packager: None,
                            arch: iref.arch().map(|s| s.to_string()).unwrap_or_default(),
                            reason: None,
                        });
                    }
                }
            }

            Err(Error::PackageNotFound(name))
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

        info!("Flatpak operation: {:?}", operation.kind);

        // Flatpak operations would need proper implementation with transactions.
        // For now, return placeholder results.
        let result = match operation.kind {
            OperationKind::Install
            | OperationKind::Remove
            | OperationKind::RemoveWithDeps
            | OperationKind::Update
            | OperationKind::SystemUpgrade => {
                warn!("Flatpak operations not fully implemented yet");
                OperationResult::failure(
                    operation,
                    "Flatpak operations require further implementation",
                    start.elapsed().as_millis() as u64,
                )
            }
            OperationKind::SyncDatabases => {
                // Flatpak doesn't have separate db sync.
                OperationResult::success(
                    operation,
                    Vec::new(),
                    start.elapsed().as_millis() as u64,
                )
            }
            OperationKind::CleanCache => {
                let freed = self.clean_cache(0).await?;
                info!("Freed {} bytes from flatpak cache", freed);
                OperationResult::success(
                    operation,
                    Vec::new(),
                    start.elapsed().as_millis() as u64,
                )
            }
            OperationKind::RemoveOrphans => {
                // Flatpak handles runtime cleanup automatically.
                OperationResult::success(
                    operation,
                    Vec::new(),
                    start.elapsed().as_millis() as u64,
                )
            }
        };

        Ok(result)
    }

    async fn sync_databases(&self) -> Result<()> {
        // Flatpak doesn't require explicit database sync.
        Ok(())
    }

    async fn get_cache_size(&self) -> Result<u64> {
        // Flatpak manages its own cache.
        Ok(0)
    }

    async fn clean_cache(&self, _keep_versions: usize) -> Result<u64> {
        tokio::task::spawn_blocking(|| {
            let installations: Vec<Installation> = [
                Self::get_user_installation().ok(),
                Self::get_system_installation().ok(),
            ]
            .into_iter()
            .flatten()
            .collect();

            for installation in installations {
                if let Err(e) = installation.prune_local_repo(gio::Cancellable::NONE) {
                    warn!("Failed to prune flatpak repo: {}", e);
                }
            }

            Ok(0u64)
        })
        .await
        .map_err(|e| Error::Other(e.to_string()))?
    }

    async fn list_orphans(&self) -> Result<Vec<Package>> {
        // Flatpak handles orphan runtimes automatically.
        Ok(Vec::new())
    }
}
