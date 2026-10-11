//! std's docs are complete (spec
//! `docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`
//! §12.2; plan decision 18).

mod common;

/// The modules whose docs are written. Each std task adds its own; the
/// last checks every module.
const DONE: &[&str] = &["core", "collections", "strings", "bytes", "fmt"];

#[test]
fn std_is_documented() {
    let public = common::public_types(&common::std_sources());
    let mut missing = Vec::new();
    for (name, text) in nova_std::ALL {
        if !DONE.contains(&name) {
            continue;
        }
        if common::module_docs(text).trim().is_empty() {
            missing.push(format!("std/{name}: no `//!` docs"));
        }
        for (what, docs) in common::needing_docs(&common::parse(text), &public) {
            if docs.iter().all(|l| l.value.trim().is_empty()) {
                missing.push(format!("std/{name}: {what}"));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "{} undocumented:\n{}",
        missing.len(),
        missing.join("\n")
    );
}
