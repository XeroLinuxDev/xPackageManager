//! Core types and traits for xPackageManager.
//!
//! This crate provides the foundational abstractions used by all backends
//! and the service layer.

pub mod error;
pub mod operation;
pub mod package;
pub mod source;

pub use error::{Error, Result};
pub use operation::{Operation, OperationKind, OperationResult, OperationStatus};
pub use package::{Package, PackageInfo, PackageStatus, SearchResult, UpdateInfo, Version};
pub use source::PackageSource;
