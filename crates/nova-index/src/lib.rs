//! The package index (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`):
//! reading an index over HTTPS, from a local directory or through the
//! GitHub API, downloading, unpacking and packing packages, the sync step,
//! and publishing. Everything nova does on the network is in this crate.
//! See ARCHITECTURE.md for its place in the pipeline.

mod cache;
mod line;
mod location;
mod pack;
mod read;

pub use cache::{unpack, unpack_limited, Limits, LIMITS};
pub use line::{parse_config, parse_lines, Config, Line, LineDep, LINE_VERSION};
pub use location::{fill_dl, index_path, Index, Location, Source, DEFAULT_INDEX};
pub use pack::{pack, sha256_hex, Packed, MAX_TARBALL};
pub use read::{LocalReader, Reader, View};
