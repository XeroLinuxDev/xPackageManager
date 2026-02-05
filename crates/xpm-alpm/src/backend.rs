//! ALPM backend implementation.

use crate::cache::CacheManager;
use crate::orphan::OrphanDetector;
use alpm::{Alpm, SigLevel};
use async_trait::async_trait;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};
use xpm_core::{
    error::{Error, Result},
    operation::{Operation, OperationKind, OperationProgress, OperationResult, OperationStatus},
    package::{
        InstallReason, Package, PackageBackend, PackageInfo, PackageStatus, SearchResult,
        UpdateInfo, Version,
    },
    source::{PackageSource, ProgressCallback},
};

/// Default paths for Arch Linux.
const DEFAULT_ROOT: &str = "/";
const DEFAULT_DBPATH: &str = "/var/lib/pacman";

/// Configuration for the ALPM backend.
#[derive(Debug, Clone)]
pub struct AlpmConfig {
    /// Root directory for package installation.
    pub root: String,
    /// Database path.
    pub dbpath: String,
    /// Cache directories.
    pub cache_dirs: Vec<String>,
    /// Hook directories.
    pub hook_dirs: Vec<String>,
    /// GPG directory.
    pub gpgdir: String,
    /// Log file path.
    pub logfile: String,
}

impl Default for AlpmConfig {
    fn default() -> Self {
        Self {
            root: DEFAULT_ROOT.to_string(),
            dbpath: DEFAULT_DBPATH.to_string(),
            cache_dirs: vec!["/var/cache/pacman/pkg".to_string()],
            hook_dirs: vec![
                "/etc/pacman.d/hooks".to_string(),
                "/usr/share/libalpm/hooks".to_string(),
            ],
            gpgdir: "/etc/pacman.d/gnupg".to_string(),
            logfile: "/var/log/pacman.log".to_string(),
        }
    }
}

/// The pacman/libalpm backend.
pub struct AlpmBackend {
    config: AlpmConfig,
    cache_manager: CacheManager,
}

// ALPM handle is not Send/Sync, so we create it on demand in blocking tasks.
unsafe impl Send for AlpmBackend {}
unsafe impl Sync for AlpmBackend {}

impl AlpmBackend {
    /// Creates a new ALPM backend with default configuration.
    pub fn new() -> Result<Self> {
        Self::with_config(AlpmConfig::default())
    }

    /// Creates a new ALPM backend with custom configuration.
    pub fn with_config(config: AlpmConfig) -> Result<Self> {
        // Verify the database path exists.
        if !Path::new(&config.dbpath).exists() {
            return Err(Error::DatabaseError(format!(
                "Database path does not exist: {}",
                config.dbpath
            )));
        }

        Ok(Self {
            cache_manager: CacheManager::new(&config.cache_dirs),
            config,
        })
    }

    /// Creates an ALPM handle (must be called from blocking context).
    fn create_handle(root: String, dbpath: String) -> Result<Alpm> {
        let handle = Alpm::new(root, dbpath)
            .map_err(|e| Error::DatabaseError(format!("Failed to initialize ALPM: {}", e)))?;

        // Register common Arch repos.
        let repos = ["core", "extra", "multilib"];
        let siglevel = SigLevel::PACKAGE_OPTIONAL | SigLevel::DATABASE_OPTIONAL;

        for repo in repos {
            if let Err(e) = handle.register_syncdb(repo, siglevel) {
                warn!("Failed to register {}: {}", repo, e);
            } else {
                debug!("Registered sync database: {}", repo);
            }
        }

        Ok(handle)
    }

    /// Converts an ALPM package to our Package type.
    fn convert_package(
        name: &str,
        version: &str,
        desc: &str,
        repo: &str,
        status: PackageStatus,
    ) -> Package {
        Package::new(
            name,
            Version::new(version),
            desc,
            PackageBackend::Pacman,
            status,
            repo,
        )
    }
}

#[async_trait]
impl PackageSource for AlpmBackend {
    fn source_id(&self) -> &str {
        "pacman"
    }

    fn display_name(&self) -> &str {
        "Pacman"
    }

    async fn is_available(&self) -> bool {
        Path::new(&self.config.dbpath).exists()
    }

    async fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        let config = self.config.clone();
        let query = query.to_string();

        tokio::task::spawn_blocking(move || {
            let handle = Alpm::new(config.root.clone(), config.dbpath.clone())
                .map_err(|e| Error::DatabaseError(e.to_string()))?;

            // Register sync databases.
            let siglevel = SigLevel::PACKAGE_OPTIONAL | SigLevel::DATABASE_OPTIONAL;
            for repo in ["core", "extra", "multilib", "xerolinux", "chaotic-aur"] {
                handle.register_syncdb(repo, siglevel).ok();
            }

            let mut results = Vec::new();
            let query_lower = query.to_lowercase();

            // Search in sync databases.
            for db in handle.syncdbs() {
                for pkg in db.pkgs() {
                    let name = pkg.name();
                    let desc = pkg.desc().unwrap_or_default();

                    if name.to_lowercase().contains(&query_lower)
                        || desc.to_lowercase().contains(&query_lower)
                    {
                        let installed = handle.localdb().pkg(name).is_ok();
                        let installed_version = if installed {
                            handle
                                .localdb()
                                .pkg(name)
                                .ok()
                                .map(|p| Version::new(p.version().as_str()))
                        } else {
                            None
                        };

                        results.push(SearchResult {
                            name: name.to_string(),
                            version: Version::new(pkg.version().as_str()),
                            description: desc.to_string(),
                            backend: PackageBackend::Pacman,
                            repository: db.name().to_string(),
                            installed,
                            installed_version,
                        });
                    }
                }
            }

            Ok(results)
        })
        .await
        .map_err(|e| Error::Other(e.to_string()))?
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let config = self.config.clone();

        tokio::task::spawn_blocking(move || {
            let handle = Alpm::new(config.root.clone(), config.dbpath.clone())
                .map_err(|e| Error::DatabaseError(e.to_string()))?;

            let mut packages = Vec::new();

            for pkg in handle.localdb().pkgs() {
                let is_orphan = pkg.reason() == alpm::PackageReason::Depend
                    && pkg.required_by().is_empty()
                    && pkg.optional_for().is_empty();

                let status = if is_orphan {
                    PackageStatus::Orphan
                } else {
                    PackageStatus::Installed
                };

                packages.push(Package::new(
                    pkg.name(),
                    Version::new(pkg.version().as_str()),
                    pkg.desc().unwrap_or_default(),
                    PackageBackend::Pacman,
                    status,
                    "local",
                ));
            }

            Ok(packages)
        })
        .await
        .map_err(|e| Error::Other(e.to_string()))?
    }

    async fn list_updates(&self) -> Result<Vec<UpdateInfo>> {
        let config = self.config.clone();

        tokio::task::spawn_blocking(move || {
            // Use checkupdates approach: sync to temp db, then compare
            // This doesn't require root privileges

            let temp_dir = std::env::temp_dir().join("xpm-checkupdates");
            let temp_dbpath = temp_dir.join("db");

            // Create temp directory structure
            std::fs::create_dir_all(&temp_dbpath).ok();

            // Symlink local database to temp location (read-only)
            let local_db_src = Path::new(&config.dbpath).join("local");
            let local_db_dst = temp_dbpath.join("local");
            if local_db_src.exists() && !local_db_dst.exists() {
                std::os::unix::fs::symlink(&local_db_src, &local_db_dst).ok();
            }

            // Create handle with temp dbpath for syncing
            let mut handle = Alpm::new(config.root.clone(), temp_dbpath.to_string_lossy().to_string())
                .map_err(|e| Error::DatabaseError(e.to_string()))?;

            // Register databases with mirror servers
            let siglevel = SigLevel::PACKAGE_OPTIONAL | SigLevel::DATABASE_OPTIONAL;
            let arch = "x86_64";
            for repo in ["core", "extra", "multilib", "xerolinux", "chaotic-aur"] {
                if let Ok(db) = handle.register_syncdb_mut(repo, siglevel) {
                    match repo {
                        "core" | "extra" | "multilib" => {
                            db.add_server(format!("https://geo.mirror.pkgbuild.com/{}/os/{}", repo, arch)).ok();
                        }
                        "chaotic-aur" => {
                            db.add_server(format!("https://geo-mirror.chaotic.cx/{}/{}", repo, arch)).ok();
                        }
                        "xerolinux" => {
                            db.add_server(format!("https://repos.xerolinux.xyz/{}/{}", repo, arch)).ok();
                        }
                        _ => {}
                    }
                }
            }

            // Sync all databases at once
            if let Err(e) = handle.syncdbs_mut().update(false) {
                warn!("Failed to sync databases: {}", e);
            }

            let mut updates = Vec::new();

            for local_pkg in handle.localdb().pkgs() {
                let name = local_pkg.name();

                // Check sync dbs for newer version.
                for db in handle.syncdbs() {
                    if let Ok(sync_pkg) = db.pkg(name) {
                        let local_ver = local_pkg.version();
                        let sync_ver = sync_pkg.version();

                        if alpm::vercmp(sync_ver.as_str(), local_ver.as_str())
                            == std::cmp::Ordering::Greater
                        {
                            updates.push(UpdateInfo {
                                name: name.to_string(),
                                current_version: Version::new(local_ver.as_str()),
                                new_version: Version::new(sync_ver.as_str()),
                                backend: PackageBackend::Pacman,
                                repository: db.name().to_string(),
                                download_size: sync_pkg.download_size() as u64,
                            });
                            break;
                        }
                    }
                }
            }

            Ok(updates)
        })
        .await
        .map_err(|e| Error::Other(e.to_string()))?
    }

    async fn get_package_info(&self, name: &str) -> Result<PackageInfo> {
        let config = self.config.clone();
        let name = name.to_string();

        tokio::task::spawn_blocking(move || {
            let handle = Alpm::new(config.root.clone(), config.dbpath.clone())
                .map_err(|e| Error::DatabaseError(e.to_string()))?;

            // Register sync databases.
            let siglevel = SigLevel::PACKAGE_OPTIONAL | SigLevel::DATABASE_OPTIONAL;
            for repo in ["core", "extra", "multilib", "xerolinux", "chaotic-aur"] {
                handle.register_syncdb(repo, siglevel).ok();
            }

            // Try local db first.
            if let Ok(pkg) = handle.localdb().pkg(name.as_bytes()) {
                let is_orphan = pkg.reason() == alpm::PackageReason::Depend
                    && pkg.required_by().is_empty()
                    && pkg.optional_for().is_empty();

                let status = if is_orphan {
                    PackageStatus::Orphan
                } else {
                    PackageStatus::Installed
                };

                let reason = Some(match pkg.reason() {
                    alpm::PackageReason::Explicit => InstallReason::Explicit,
                    alpm::PackageReason::Depend => InstallReason::Dependency,
                });

                return Ok(PackageInfo {
                    package: Package::new(
                        pkg.name(),
                        Version::new(pkg.version().as_str()),
                        pkg.desc().unwrap_or_default(),
                        PackageBackend::Pacman,
                        status,
                        "local",
                    ),
                    url: pkg.url().map(|s| s.to_string()),
                    licenses: pkg.licenses().iter().map(|s| s.to_string()).collect(),
                    groups: pkg.groups().iter().map(|s| s.to_string()).collect(),
                    depends: pkg.depends().iter().map(|d| d.to_string()).collect(),
                    optdepends: pkg.optdepends().iter().map(|d| d.to_string()).collect(),
                    provides: pkg.provides().iter().map(|d| d.to_string()).collect(),
                    conflicts: pkg.conflicts().iter().map(|d| d.to_string()).collect(),
                    replaces: pkg.replaces().iter().map(|d| d.to_string()).collect(),
                    installed_size: pkg.isize() as u64,
                    download_size: pkg.download_size() as u64,
                    build_date: Some(
                        chrono::DateTime::from_timestamp(pkg.build_date(), 0)
                            .unwrap_or_default()
                            .with_timezone(&chrono::Utc),
                    ),
                    install_date: pkg.install_date().map(|ts| {
                        chrono::DateTime::from_timestamp(ts, 0)
                            .unwrap_or_default()
                            .with_timezone(&chrono::Utc)
                    }),
                    packager: pkg.packager().map(|s| s.to_string()),
                    arch: pkg.arch().unwrap_or("any").to_string(),
                    reason,
                });
            }

            // Try sync dbs.
            for db in handle.syncdbs() {
                if let Ok(pkg) = db.pkg(name.as_bytes()) {
                    return Ok(PackageInfo {
                        package: Package::new(
                            pkg.name(),
                            Version::new(pkg.version().as_str()),
                            pkg.desc().unwrap_or_default(),
                            PackageBackend::Pacman,
                            PackageStatus::Available,
                            db.name(),
                        ),
                        url: pkg.url().map(|s| s.to_string()),
                        licenses: pkg.licenses().iter().map(|s| s.to_string()).collect(),
                        groups: pkg.groups().iter().map(|s| s.to_string()).collect(),
                        depends: pkg.depends().iter().map(|d| d.to_string()).collect(),
                        optdepends: pkg.optdepends().iter().map(|d| d.to_string()).collect(),
                        provides: pkg.provides().iter().map(|d| d.to_string()).collect(),
                        conflicts: pkg.conflicts().iter().map(|d| d.to_string()).collect(),
                        replaces: pkg.replaces().iter().map(|d| d.to_string()).collect(),
                        installed_size: pkg.isize() as u64,
                        download_size: pkg.download_size() as u64,
                        build_date: Some(
                            chrono::DateTime::from_timestamp(pkg.build_date(), 0)
                                .unwrap_or_default()
                                .with_timezone(&chrono::Utc),
                        ),
                        install_date: None,
                        packager: pkg.packager().map(|s| s.to_string()),
                        arch: pkg.arch().unwrap_or("any").to_string(),
                        reason: None,
                    });
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

        info!("Executing operation: {:?}", operation.kind);

        // For actual package operations, we would need to call pacman/libalpm
        // with root privileges. For now, return a placeholder result.
        let result = match operation.kind {
            OperationKind::Install
            | OperationKind::Remove
            | OperationKind::RemoveWithDeps
            | OperationKind::Update
            | OperationKind::SystemUpgrade => {
                // TODO: Implement actual package operations via polkit/pkexec.
                warn!(
                    "Package operations require root privileges - not implemented yet"
                );
                OperationResult::failure(
                    operation,
                    "Package operations require root privileges",
                    start.elapsed().as_millis() as u64,
                )
            }
            OperationKind::SyncDatabases => {
                // Database sync also needs root for system-wide sync.
                warn!("Database sync requires root privileges");
                OperationResult::failure(
                    operation,
                    "Database sync requires root privileges",
                    start.elapsed().as_millis() as u64,
                )
            }
            OperationKind::CleanCache => {
                let freed = self.cache_manager.clean(3).await?;
                info!("Freed {} bytes from cache", freed);
                OperationResult::success(operation, Vec::new(), start.elapsed().as_millis() as u64)
            }
            OperationKind::RemoveOrphans => {
                let orphans = self.list_orphans().await?;
                if orphans.is_empty() {
                    OperationResult::success(
                        operation,
                        Vec::new(),
                        start.elapsed().as_millis() as u64,
                    )
                } else {
                    OperationResult::failure(
                        operation,
                        "Removing orphans requires root privileges",
                        start.elapsed().as_millis() as u64,
                    )
                }
            }
        };

        Ok(result)
    }

    async fn sync_databases(&self) -> Result<()> {
        // Database sync requires root privileges.
        warn!("Database sync requires root privileges - skipping");
        Ok(())
    }

    async fn get_cache_size(&self) -> Result<u64> {
        self.cache_manager.get_size().await
    }

    async fn clean_cache(&self, keep_versions: usize) -> Result<u64> {
        self.cache_manager.clean(keep_versions).await
    }

    async fn list_orphans(&self) -> Result<Vec<Package>> {
        let config = self.config.clone();

        tokio::task::spawn_blocking(move || {
            let handle = Alpm::new(config.root.clone(), config.dbpath.clone())
                .map_err(|e| Error::DatabaseError(e.to_string()))?;

            let mut orphans = Vec::new();

            for pkg in handle.localdb().pkgs() {
                let is_orphan = pkg.reason() == alpm::PackageReason::Depend
                    && pkg.required_by().is_empty()
                    && pkg.optional_for().is_empty();

                if is_orphan {
                    orphans.push(Package::new(
                        pkg.name(),
                        Version::new(pkg.version().as_str()),
                        pkg.desc().unwrap_or_default(),
                        PackageBackend::Pacman,
                        PackageStatus::Orphan,
                        "local",
                    ));
                }
            }

            Ok(orphans)
        })
        .await
        .map_err(|e| Error::Other(e.to_string()))?
    }
}
