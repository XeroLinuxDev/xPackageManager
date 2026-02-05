//! Orphan package detection.

use alpm::{Package, PackageReason};

/// Detects orphan packages (installed as dependencies but no longer needed).
pub struct OrphanDetector;

impl OrphanDetector {
    /// Creates a new orphan detector.
    pub fn new() -> Self {
        Self
    }

    /// Checks if a package is an orphan.
    ///
    /// A package is considered orphan if:
    /// - It was installed as a dependency (not explicitly)
    /// - No other installed package depends on it
    pub fn is_orphan(&self, pkg: &Package) -> bool {
        // Must be installed as a dependency.
        if pkg.reason() != PackageReason::Depend {
            return false;
        }

        // Check if any other package requires this one.
        pkg.required_by().is_empty() && pkg.optional_for().is_empty()
    }
}

impl Default for OrphanDetector {
    fn default() -> Self {
        Self::new()
    }
}
