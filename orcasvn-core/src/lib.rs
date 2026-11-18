//! Shared SVN backend for the OrcaSVN clients.
//!
//! The module tree is extracted from the Tauri client (`src-tauri/src/svn`) so
//! the Tauri, egui and GPUI front ends can run the same, tested implementation.

pub mod gitignore;
pub mod svn;
mod types;

pub use types::*;
