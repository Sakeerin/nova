//! The package index (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`):
//! reading an index over HTTPS, from a local directory or through the
//! GitHub API, downloading, unpacking and packing packages, the sync step,
//! and publishing. Everything nova does on the network is in this crate.
//! See ARCHITECTURE.md for its place in the pipeline.

mod cache;
mod download;
mod http;
mod line;
mod location;
mod pack;
mod read;
mod sync;

pub use cache::{unpack, unpack_limited, Limits, LIMITS};
pub use download::fetch_package;
pub use http::{check_url, Http, MAX_INDEX_FILE};
pub use line::{parse_config, parse_lines, Config, Line, LineDep, LINE_VERSION};
pub use location::{fill_dl, index_path, Index, Location, Source, DEFAULT_INDEX};
pub use pack::{pack, sha256_hex, Packed, MAX_TARBALL};
pub use read::{reader_for, HttpReader, LocalReader, Reader, View};
pub use sync::{sync, write_lock, SyncError, SyncRequest, Synced};
