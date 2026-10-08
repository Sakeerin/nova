//! The package manager's foundations (Phase 3.0): `nova.toml` parsing,
//! package names, and finding a project (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §5
//! and §6.1). See ARCHITECTURE.md for the crate's place in the pipeline.

pub mod manifest;
mod name;
mod project;

pub use manifest::{parse, Dependency, Manifest, Package};
pub use name::{check_name, import_name};
pub use project::{find_root, real_path, MANIFEST};
