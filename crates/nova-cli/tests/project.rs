//! End-to-end tests of nova's project model (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`):
//! `nova version`, project discovery, and `nova new` and `nova init`.
//!
//! Every test works in its own fresh directory under the system temp
//! directory, and writes a `nova.toml` only inside it. Project discovery
//! walks up from the current directory, so a stray `nova.toml` in a shared
//! directory would capture every test run below it.

use assert_cmd::Command;

fn nova() -> Command {
    Command::cargo_bin("nova").expect("nova binary builds")
}

#[test]
fn version_reports_the_build_and_whether_the_runtime_is_embedded() {
    // build.rs exports the payload's size to every target of the package,
    // so this holds whether or not NOVA_EMBED_RUNTIME was set.
    let embedded = env!("NOVA_EMBEDDED_RUNTIME_SIZE") != "0";
    let expected = format!(
        "nova {}\ntarget: {}\nruntime: {}\n",
        env!("CARGO_PKG_VERSION"),
        env!("NOVA_TARGET"),
        if embedded { "embedded" } else { "not embedded" }
    );
    nova().arg("version").assert().success().stdout(expected);
}
