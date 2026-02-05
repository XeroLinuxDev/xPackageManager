//! Package and version types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;

/// Represents a package version with comparison support.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Version {
    /// The full version string (e.g., "1.2.3-1").
    pub full: String,
    /// Epoch (if any).
    pub epoch: Option<u32>,
    /// Package version (upstream).
    pub pkgver: String,
    /// Package release number.
    pub pkgrel: String,
}

impl Version {
    /// Creates a new Version from a version string.
    pub fn new(version_str: &str) -> Self {
        let (epoch, rest) = if let Some(idx) = version_str.find(':') {
            let epoch = version_str[..idx].parse().ok();
            (epoch, &version_str[idx + 1..])
        } else {
            (None, version_str)
        };

        let (pkgver, pkgrel) = if let Some(idx) = rest.rfind('-') {
            (rest[..idx].to_string(), rest[idx + 1..].to_string())
        } else {
            (rest.to_string(), String::new())
        };

        Self {
            full: version_str.to_string(),
            epoch,
            pkgver,
            pkgrel,
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.full)
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare epochs first
        match (self.epoch, other.epoch) {
            (Some(a), Some(b)) => match a.cmp(&b) {
                Ordering::Equal => {}
                ord => return ord,
            },
            (Some(_), None) => return Ordering::Greater,
            (None, Some(_)) => return Ordering::Less,
            (None, None) => {}
        }

        // Then compare pkgver using version comparison
        match vercmp(&self.pkgver, &other.pkgver) {
            Ordering::Equal => {}
            ord => return ord,
        }

        // Finally compare pkgrel
        vercmp(&self.pkgrel, &other.pkgrel)
    }
}

/// Simple version comparison (alphanumeric segments).
fn vercmp(a: &str, b: &str) -> Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        // Skip non-alphanumeric
        while a_chars.peek().is_some_and(|c| !c.is_alphanumeric()) {
            a_chars.next();
        }
        while b_chars.peek().is_some_and(|c| !c.is_alphanumeric()) {
            b_chars.next();
        }

        match (a_chars.peek().copied(), b_chars.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ac), Some(bc)) => {
                let a_is_digit = ac.is_ascii_digit();
                let b_is_digit = bc.is_ascii_digit();

                match (a_is_digit, b_is_digit) {
                    (true, true) => {
                        // Compare numeric segments
                        let mut a_num = String::new();
                        while let Some(&c) = a_chars.peek() {
                            if c.is_ascii_digit() {
                                a_num.push(c);
                                a_chars.next();
                            } else {
                                break;
                            }
                        }
                        let mut b_num = String::new();
                        while let Some(&c) = b_chars.peek() {
                            if c.is_ascii_digit() {
                                b_num.push(c);
                                b_chars.next();
                            } else {
                                break;
                            }
                        }

                        // Compare as numbers (longer numeric string is greater, or lexicographic)
                        match a_num.len().cmp(&b_num.len()) {
                            Ordering::Equal => match a_num.cmp(&b_num) {
                                Ordering::Equal => continue,
                                ord => return ord,
                            },
                            ord => return ord,
                        }
                    }
                    (false, false) => {
                        // Compare alphabetic segments
                        let mut a_alpha = String::new();
                        while let Some(&c) = a_chars.peek() {
                            if c.is_alphabetic() {
                                a_alpha.push(c);
                                a_chars.next();
                            } else {
                                break;
                            }
                        }
                        let mut b_alpha = String::new();
                        while let Some(&c) = b_chars.peek() {
                            if c.is_alphabetic() {
                                b_alpha.push(c);
                                b_chars.next();
                            } else {
                                break;
                            }
                        }

                        match a_alpha.cmp(&b_alpha) {
                            Ordering::Equal => continue,
                            ord => return ord,
                        }
                    }
                    (true, false) => return Ordering::Greater, // Numbers > letters
                    (false, true) => return Ordering::Less,
                }
            }
        }
    }
}

/// The source/backend a package comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PackageBackend {
    /// Pacman/libalpm (Arch repos + AUR).
    Pacman,
    /// Flatpak.
    Flatpak,
}

impl fmt::Display for PackageBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageBackend::Pacman => write!(f, "pacman"),
            PackageBackend::Flatpak => write!(f, "flatpak"),
        }
    }
}

/// Installation status of a package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PackageStatus {
    /// Package is installed.
    Installed,
    /// Package is available but not installed.
    Available,
    /// Package is installed but an update is available.
    Upgradable,
    /// Package is orphaned (no longer needed by any other package).
    Orphan,
}

/// Core package representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    /// Package name.
    pub name: String,
    /// Currently installed or available version.
    pub version: Version,
    /// Package description.
    pub description: String,
    /// Which backend this package belongs to.
    pub backend: PackageBackend,
    /// Current status.
    pub status: PackageStatus,
    /// Repository name (e.g., "extra", "flathub").
    pub repository: String,
}

impl Package {
    /// Creates a new Package.
    pub fn new(
        name: impl Into<String>,
        version: Version,
        description: impl Into<String>,
        backend: PackageBackend,
        status: PackageStatus,
        repository: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            version,
            description: description.into(),
            backend,
            status,
            repository: repository.into(),
        }
    }
}

/// Extended package information for details view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    /// Base package data.
    pub package: Package,
    /// Package URL/homepage.
    pub url: Option<String>,
    /// License(s).
    pub licenses: Vec<String>,
    /// Package groups.
    pub groups: Vec<String>,
    /// Direct dependencies.
    pub depends: Vec<String>,
    /// Optional dependencies.
    pub optdepends: Vec<String>,
    /// Packages this provides.
    pub provides: Vec<String>,
    /// Packages this conflicts with.
    pub conflicts: Vec<String>,
    /// Packages this replaces.
    pub replaces: Vec<String>,
    /// Installed size in bytes.
    pub installed_size: u64,
    /// Download size in bytes.
    pub download_size: u64,
    /// Build/package date.
    pub build_date: Option<DateTime<Utc>>,
    /// Install date (if installed).
    pub install_date: Option<DateTime<Utc>>,
    /// Packager.
    pub packager: Option<String>,
    /// Architecture.
    pub arch: String,
    /// Install reason (explicit or dependency).
    pub reason: Option<InstallReason>,
}

/// Why a package was installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InstallReason {
    /// Explicitly installed by user.
    Explicit,
    /// Installed as a dependency.
    Dependency,
}

/// Search result from a backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Package name.
    pub name: String,
    /// Package version.
    pub version: Version,
    /// Package description.
    pub description: String,
    /// Which backend this result is from.
    pub backend: PackageBackend,
    /// Repository name.
    pub repository: String,
    /// Whether this package is currently installed.
    pub installed: bool,
    /// Installed version (if different from available).
    pub installed_version: Option<Version>,
}

/// Information about an available update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    /// Package name.
    pub name: String,
    /// Currently installed version.
    pub current_version: Version,
    /// New available version.
    pub new_version: Version,
    /// Which backend.
    pub backend: PackageBackend,
    /// Repository name.
    pub repository: String,
    /// Download size for the update.
    pub download_size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        let v1 = Version::new("1.0.0-1");
        let v2 = Version::new("1.0.1-1");
        let v3 = Version::new("1:0.5.0-1");
        let v4 = Version::new("2.0.0-1");

        assert!(v1 < v2);
        assert!(v2 < v4);
        assert!(v3 > v4); // epoch wins
        assert!(v1 == Version::new("1.0.0-1"));
    }

    #[test]
    fn test_version_parsing() {
        let v = Version::new("1:2.3.4-5");
        assert_eq!(v.epoch, Some(1));
        assert_eq!(v.pkgver, "2.3.4");
        assert_eq!(v.pkgrel, "5");
    }
}
