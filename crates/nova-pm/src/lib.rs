//! The package manager: `nova.toml` parsing, package names, finding a
//! project, and the package graph (Phase 3.0 and 3.3a; specs
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §5
//! and §6.1, and
//! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
//! §3), and the index's offline side: `$NOVA_HOME`, `nova.lock` and the
//! resolver (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §4, §5). See ARCHITECTURE.md for the crate's place in the pipeline.

mod graph;
mod home;
mod index_name;
mod lock;
pub mod manifest;
mod name;
mod project;
mod resolve;

pub use graph::{
    graph, graph_from, graph_with, requirements, Edge, Graph, GraphPackage, Offline, PackageId,
};
pub use home::{nova_home, nova_home_from_env, registry_dir};
pub use index_name::{canonical_index, index_dir_name, local_index_path};
pub use lock::{parse_lock, Lock, LockedPackage, LOCKFILE};
pub use manifest::{parse, Dependency, Manifest, Package};
pub use name::{check_name, import_name, is_portable};
pub use project::{find_root, real_path, MANIFEST};
pub use resolve::{resolve, Candidate, IndexView, Requirement, ResolveError, Unlock};
