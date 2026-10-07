//! `nova version`: the version, the target `nova` was built for, and
//! whether it carries its runtime library (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`
//! §4.3). clap's `--version` prints only the first of these.

pub fn run() -> anyhow::Result<()> {
    print!("{}", text(!crate::embedded::PAYLOAD.is_empty()));
    Ok(())
}

fn text(embedded: bool) -> String {
    format!(
        "nova {}\ntarget: {}\nruntime: {}\n",
        env!("CARGO_PKG_VERSION"),
        env!("NOVA_TARGET"),
        if embedded { "embedded" } else { "not embedded" }
    )
}
