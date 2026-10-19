//! What `nova dev` serves (the 4.5 spec
//! `docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`
//! §4.4, §4.5): the HTTP server and its event stream, with the reload script
//! it adds to each page. The command itself, its builds and its watcher are
//! `crate::cmd::dev`.

pub mod server;
