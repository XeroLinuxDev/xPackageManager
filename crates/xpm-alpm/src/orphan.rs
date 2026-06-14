use alpm::{Package, PackageReason};

pub struct OrphanDetector;

impl OrphanDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn is_orphan(&self, pkg: &Package) -> bool {
        if pkg.reason() != PackageReason::Depend {
            return false;
        }

        pkg.required_by().is_empty() && pkg.optional_for().is_empty()
    }
}

impl Default for OrphanDetector {
    fn default() -> Self {
        Self::new()
    }
}
