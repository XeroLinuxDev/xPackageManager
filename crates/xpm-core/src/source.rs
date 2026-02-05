//! Package source trait defining the backend interface.

use crate::error::Result;
use crate::operation::{Operation, OperationProgress, OperationResult};
use crate::package::{Package, PackageInfo, SearchResult, UpdateInfo};
use async_trait::async_trait;

/// Callback type for progress updates.
pub type ProgressCallback = Box<dyn Fn(OperationProgress) + Send + Sync>;

/// Trait implemented by all package backends (pacman, flatpak, etc.).
#[async_trait]
pub trait PackageSource: Send + Sync {
    /// Returns the unique identifier for this source (e.g., "pacman", "flatpak").
    fn source_id(&self) -> &str;

    /// Returns a human-readable name for this source.
    fn display_name(&self) -> &str;

    /// Checks if this backend is available on the system.
    async fn is_available(&self) -> bool;

    /// Searches for packages matching the query.
    async fn search(&self, query: &str) -> Result<Vec<SearchResult>>;

    /// Lists all installed packages from this source.
    async fn list_installed(&self) -> Result<Vec<Package>>;

    /// Lists packages with available updates.
    async fn list_updates(&self) -> Result<Vec<UpdateInfo>>;

    /// Gets detailed information about a specific package.
    async fn get_package_info(&self, name: &str) -> Result<PackageInfo>;

    /// Executes a package operation.
    async fn execute(&self, operation: Operation) -> Result<OperationResult>;

    /// Executes a package operation with progress reporting.
    async fn execute_with_progress(
        &self,
        operation: Operation,
        progress: ProgressCallback,
    ) -> Result<OperationResult>;

    /// Synchronizes package databases (if applicable).
    async fn sync_databases(&self) -> Result<()>;

    /// Gets the total cache size in bytes.
    async fn get_cache_size(&self) -> Result<u64>;

    /// Cleans the package cache.
    async fn clean_cache(&self, keep_versions: usize) -> Result<u64>;

    /// Lists orphan packages (installed as deps but no longer needed).
    async fn list_orphans(&self) -> Result<Vec<Package>>;
}

/// Extension trait for common operations across sources.
#[async_trait]
pub trait PackageSourceExt: PackageSource {
    /// Checks if a package is installed.
    async fn is_installed(&self, name: &str) -> Result<bool> {
        let installed = self.list_installed().await?;
        Ok(installed.iter().any(|p| p.name == name))
    }

    /// Counts installed packages.
    async fn installed_count(&self) -> Result<usize> {
        Ok(self.list_installed().await?.len())
    }

    /// Counts available updates.
    async fn update_count(&self) -> Result<usize> {
        Ok(self.list_updates().await?.len())
    }
}

// Blanket implementation for all PackageSources.
impl<T: PackageSource> PackageSourceExt for T {}
