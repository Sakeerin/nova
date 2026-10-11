//! std's docs are complete (spec
//! `docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`
//! §12.2; plan decision 18).

mod common;

#[test]
fn std_is_documented() {
    let public = common::public_types(&common::std_sources());
    let mut missing = Vec::new();
    for (name, text) in nova_std::ALL {
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

/// The modules whose examples are written (plan Tasks 11 to 13); Task 14
/// deletes this, and the test then checks every module.
const EXAMPLED: &[&str] = &["core", "collections", "strings", "bytes", "fmt"];

/// Each std module's `//!` docs hold an example that runs (spec 3.5b §9.2).
#[test]
fn every_std_module_has_an_example() {
    let mut missing = Vec::new();
    for (name, text) in nova_std::ALL {
        if !EXAMPLED.contains(&name) {
            continue;
        }
        let tested = nova_doc::code_blocks(&common::module_docs(text))
            .iter()
            .any(|b| matches!(b.block, nova_doc::Block::Test { .. }));
        if !tested {
            missing.push(format!("std/{name}"));
        }
    }
    assert!(
        missing.is_empty(),
        "{} module(s) without an example in their `//!` docs: {}",
        missing.len(),
        missing.join(", ")
    );
}
