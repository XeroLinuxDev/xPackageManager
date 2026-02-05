//! Package operations (install, remove, update, etc.).

use crate::package::{Package, PackageBackend};
use serde::{Deserialize, Serialize};
use std::fmt;

/// The kind of operation to perform.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperationKind {
    /// Install a package.
    Install,
    /// Remove a package.
    Remove,
    /// Remove a package and its unneeded dependencies.
    RemoveWithDeps,
    /// Update a specific package.
    Update,
    /// Update all packages (system upgrade).
    SystemUpgrade,
    /// Sync package databases.
    SyncDatabases,
    /// Clean package cache.
    CleanCache,
    /// Remove orphan packages.
    RemoveOrphans,
}

impl fmt::Display for OperationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperationKind::Install => write!(f, "Install"),
            OperationKind::Remove => write!(f, "Remove"),
            OperationKind::RemoveWithDeps => write!(f, "Remove with dependencies"),
            OperationKind::Update => write!(f, "Update"),
            OperationKind::SystemUpgrade => write!(f, "System upgrade"),
            OperationKind::SyncDatabases => write!(f, "Sync databases"),
            OperationKind::CleanCache => write!(f, "Clean cache"),
            OperationKind::RemoveOrphans => write!(f, "Remove orphans"),
        }
    }
}

/// A package operation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    /// What kind of operation.
    pub kind: OperationKind,
    /// Target packages (names).
    pub packages: Vec<String>,
    /// Which backend to use.
    pub backend: PackageBackend,
    /// Operation options.
    pub options: OperationOptions,
}

impl Operation {
    /// Creates a new install operation.
    pub fn install(packages: Vec<String>, backend: PackageBackend) -> Self {
        Self {
            kind: OperationKind::Install,
            packages,
            backend,
            options: OperationOptions::default(),
        }
    }

    /// Creates a new remove operation.
    pub fn remove(packages: Vec<String>, backend: PackageBackend) -> Self {
        Self {
            kind: OperationKind::Remove,
            packages,
            backend,
            options: OperationOptions::default(),
        }
    }

    /// Creates a new update operation.
    pub fn update(packages: Vec<String>, backend: PackageBackend) -> Self {
        Self {
            kind: OperationKind::Update,
            packages,
            backend,
            options: OperationOptions::default(),
        }
    }

    /// Creates a system upgrade operation.
    pub fn system_upgrade(backend: PackageBackend) -> Self {
        Self {
            kind: OperationKind::SystemUpgrade,
            packages: Vec::new(),
            backend,
            options: OperationOptions::default(),
        }
    }

    /// Creates a database sync operation.
    pub fn sync_databases(backend: PackageBackend) -> Self {
        Self {
            kind: OperationKind::SyncDatabases,
            packages: Vec::new(),
            backend,
            options: OperationOptions::default(),
        }
    }

    /// Sets the options for this operation.
    pub fn with_options(mut self, options: OperationOptions) -> Self {
        self.options = options;
        self
    }
}

/// Options that modify operation behavior.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OperationOptions {
    /// Don't ask for confirmation.
    pub no_confirm: bool,
    /// Download only, don't install.
    pub download_only: bool,
    /// Force operation even if it would break dependencies.
    pub force: bool,
    /// Also remove unneeded dependencies (for Remove).
    pub recursive: bool,
    /// Keep configuration files (for Remove).
    pub keep_config: bool,
    /// Ignore dependency version requirements.
    pub no_deps: bool,
}

/// Status of an ongoing or completed operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationStatus {
    /// Operation is pending/queued.
    Pending,
    /// Resolving dependencies.
    ResolvingDeps,
    /// Downloading packages.
    Downloading,
    /// Verifying package integrity.
    Verifying,
    /// Installing/removing packages.
    Processing,
    /// Running post-transaction hooks.
    RunningHooks,
    /// Operation completed successfully.
    Completed,
    /// Operation failed.
    Failed,
    /// Operation was cancelled.
    Cancelled,
}

/// Result of a completed operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    /// The operation that was performed.
    pub operation: Operation,
    /// Final status.
    pub status: OperationStatus,
    /// Packages that were affected.
    pub affected_packages: Vec<Package>,
    /// Any warnings generated.
    pub warnings: Vec<String>,
    /// Error message if failed.
    pub error: Option<String>,
    /// Duration in milliseconds.
    pub duration_ms: u64,
}

impl OperationResult {
    /// Creates a successful result.
    pub fn success(operation: Operation, affected: Vec<Package>, duration_ms: u64) -> Self {
        Self {
            operation,
            status: OperationStatus::Completed,
            affected_packages: affected,
            warnings: Vec::new(),
            error: None,
            duration_ms,
        }
    }

    /// Creates a failed result.
    pub fn failure(operation: Operation, error: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            operation,
            status: OperationStatus::Failed,
            affected_packages: Vec::new(),
            warnings: Vec::new(),
            error: Some(error.into()),
            duration_ms,
        }
    }

    /// Returns true if the operation was successful.
    pub fn is_success(&self) -> bool {
        self.status == OperationStatus::Completed
    }

    /// Adds a warning to the result.
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }
}

/// Progress information for an operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationProgress {
    /// Current status.
    pub status: OperationStatus,
    /// Current package being processed.
    pub current_package: Option<String>,
    /// Total packages in this operation.
    pub total_packages: usize,
    /// Packages completed so far.
    pub completed_packages: usize,
    /// Total bytes to download.
    pub total_bytes: u64,
    /// Bytes downloaded so far.
    pub downloaded_bytes: u64,
    /// Current status message.
    pub message: String,
}

impl OperationProgress {
    /// Creates a new progress tracker.
    pub fn new(total_packages: usize, total_bytes: u64) -> Self {
        Self {
            status: OperationStatus::Pending,
            current_package: None,
            total_packages,
            completed_packages: 0,
            total_bytes,
            downloaded_bytes: 0,
            message: String::new(),
        }
    }

    /// Returns the download progress as a percentage (0-100).
    pub fn download_percent(&self) -> u8 {
        if self.total_bytes == 0 {
            return 100;
        }
        ((self.downloaded_bytes as f64 / self.total_bytes as f64) * 100.0) as u8
    }

    /// Returns the package progress as a percentage (0-100).
    pub fn package_percent(&self) -> u8 {
        if self.total_packages == 0 {
            return 100;
        }
        ((self.completed_packages as f64 / self.total_packages as f64) * 100.0) as u8
    }
}
