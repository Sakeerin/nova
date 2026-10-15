//! `app.js` and `index.html` (spec
//! `docs/superpowers/specs/2026-10-14-phase-4-0-foundations-design.md`
//! §6, §7). `app.js` is the shim's core, then `env`, holding the host
//! functions the module imports and no others, then the runner.

use anyhow::{bail, Result};

/// The page `dist/` holds: it loads `app.js` as a module and calls `init`.
pub const INDEX_HTML: &str = include_str!("shim/index.html");

const CORE: &str = include_str!("shim/core.js");
const RUN: &str = include_str!("shim/run.js");

/// A host function's entry in `env`, by its import name.
fn host(name: &str) -> Result<&'static str> {
    Ok(match name {
        "print" => "  print: (s) => write(1, text(s)),\n",
        "println" => "  println: (s) => write(1, text(s) + \"\\n\"),\n",
        "eprint" => "  eprint: (s) => write(2, text(s)),\n",
        "eprintln" => "  eprintln: (s) => write(2, text(s) + \"\\n\"),\n",
        "panic" => {
            "  panic: (s) => {\n    write(2, `nova: panic: ${text(s)}\\n`);\n    throw new NovaPanic();\n  },\n"
        }
        other => bail!("the shim has no host function `{other}`"),
    })
}

/// `app.js` for a module importing `imports` from `env`.
pub fn shim(imports: &[&str]) -> Result<String> {
    let mut js = String::from(CORE);
    js.push_str("\n// The host functions the module imports.\nconst env = {\n");
    for name in imports {
        js.push_str(host(name)?);
    }
    js.push_str("};\n");
    js.push_str(RUN);
    Ok(js)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only the imported host functions are in `env`.
    #[test]
    fn env_holds_only_the_imported_host_functions() {
        let js = shim(&["println"]).expect("a shim");
        assert!(js.contains("  println: (s) =>"), "{js}");
        assert!(!js.contains("  print: (s) =>"), "{js}");
        assert!(!js.contains("  panic: (s) =>"), "{js}");
    }

    /// An import the shim cannot provide is refused, not left undefined.
    #[test]
    fn an_unknown_import_is_refused() {
        let e = shim(&["fetch"]).expect_err("refused");
        assert!(e.to_string().contains("no host function `fetch`"), "{e}");
    }
}
