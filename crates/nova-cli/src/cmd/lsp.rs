//! `nova lsp`: the language server, over stdin and stdout (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §6).

/// Serve until the client says `exit`, and return the exit code: 0 after
/// `shutdown` then `exit`, and 1 otherwise.
pub fn run() -> i32 {
    match nova_lsp::run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: nova lsp: {e:#}");
            1
        }
    }
}
