//! Pacman/libalpm backend for xPackageManager.

pub mod backend;
pub mod cache;
pub mod orphan;
pub mod transaction;

pub use backend::AlpmBackend;
