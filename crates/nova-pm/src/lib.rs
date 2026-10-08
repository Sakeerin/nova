//! The package manager: `nova.toml` parsing, package names, finding a
//! project, and the package graph (Phase 3.0 and 3.3a; specs
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §5
//! and §6.1, and
//! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
//! §3). See ARCHITECTURE.md for the crate's place in the pipeline.

mod graph;
pub mod manifest;
mod name;
mod project;

pub use graph::{graph, graph_from, Edge, Graph, GraphPackage, PackageId};
pub use manifest::{parse, Dependency, Manifest, Package};
pub use name::{check_name, import_name};
pub use project::{find_root, real_path, MANIFEST};
