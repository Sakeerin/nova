//! `app.js` and `index.html` (spec
//! `docs/superpowers/specs/2026-10-14-phase-4-0-foundations-design.md`
//! §6, §7). `app.js` is the shim's core, then `env`, holding the host
//! functions the module imports and no others, then the runner.

use anyhow::{bail, Result};

/// The page `dist/` holds: it loads `app.js` as a module and calls `init`.
pub const INDEX_HTML: &str = include_str!("shim/index.html");

/// `dist/package.json`: `app.js` is an ES module, whatever a
/// `package.json` above `dist/` says, so `node dist/app.js` reads it as
/// one. Browsers do not read it.
pub const PACKAGE_JSON: &str = "{\"type\":\"module\"}\n";

const CORE: &str = include_str!("shim/core.js");
const RUN: &str = include_str!("shim/run.js");
const FLOAT: &str = include_str!("shim/float.js");

/// `int_hash_seed`'s value: drawn once, then the same for the rest of the
/// run, as the native runtime's is.
const SEED: &str = "
// The hash seed, drawn once per run.
let seed;
function hashSeed() {
  if (seed === undefined) {
    const word = new BigUint64Array(1);
    crypto.getRandomValues(word);
    seed = BigInt.asIntN(64, word[0]);
  }
  return seed;
}
";

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
        "float_to_str" => "  float_to_str: (v) => string(floatText(v)),\n",
        "float_fixed" => "  float_fixed: (v, places) => string(floatFixed(v, places)),\n",
        "str_to_float" => "  str_to_float: (s) => floatFrom(text(s)),\n",
        "str_to_upper" => "  str_to_upper: (s) => string(text(s).toUpperCase()),\n",
        "str_to_lower" => "  str_to_lower: (s) => string(text(s).toLowerCase()),\n",
        "int_hash_seed" => "  int_hash_seed: () => hashSeed(),\n",
        other => bail!("the shim has no host function `{other}`"),
    })
}

/// `app.js` for a module importing `imports` from `env`.
pub fn shim(imports: &[&str]) -> Result<String> {
    let mut js = String::from(CORE);
    let uses = |names: &[&str]| imports.iter().any(|i| names.contains(i));
    if uses(&["float_to_str", "float_fixed", "str_to_float"]) {
        js.push_str(FLOAT);
    }
    if uses(&["int_hash_seed"]) {
        js.push_str(SEED);
    }
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

    /// The float algorithms and the seed come only with the functions that
    /// use them.
    #[test]
    fn helpers_come_only_with_their_host_functions() {
        let plain = shim(&["println"]).expect("a shim");
        assert!(!plain.contains("function floatText"), "{plain}");
        assert!(!plain.contains("function hashSeed"), "{plain}");
        let floats = shim(&["float_fixed", "int_hash_seed"]).expect("a shim");
        assert!(floats.contains("function floatFixed"), "{floats}");
        assert!(floats.contains("function hashSeed"), "{floats}");
    }

    /// An import the shim cannot provide is refused, not left undefined.
    #[test]
    fn an_unknown_import_is_refused() {
        let e = shim(&["fetch"]).expect_err("refused");
        assert!(e.to_string().contains("no host function `fetch`"), "{e}");
    }

    /// `floatFrom`, which `str_to_float` calls, is Rust's
    /// `str::parse::<f64>` with `unwrap_or(0.0)` on every case of its
    /// grammar, valid and not (spec §6, §9.3). `str_to_float` is std's
    /// alone, so no Nova program can hand it these texts: the test runs the
    /// shim's own code under Node and compares the bits.
    #[test]
    fn float_from_is_rusts_parse() {
        let Some(mut node) = crate::test_node() else {
            return;
        };
        let cases = [
            // Digits, a point, an exponent.
            "0",
            "-0",
            "+0",
            "00",
            "007",
            "1",
            "-1",
            "+1",
            "1.",
            ".5",
            "-.5",
            "+.5e-3",
            "1.5",
            "1.e1",
            "1e5",
            "1E5",
            "1e+5",
            "1e-5",
            "1e05",
            "0e0",
            "123.456e-2",
            "1e0000000000000000000005",
            // Rounding: a tie to even, the edges of the range.
            "9007199254740993",
            "0.1",
            "0.1000000000000000055511151231257827021181583404541015625",
            "1.7976931348623157e308",
            "1.7976931348623159e308",
            "1e400",
            "1e99999999999999999999",
            "4.9e-324",
            "2.4703282292062328e-324",
            "2.4703282292062327e-324",
            "1e-400",
            "0e99999999999999999999",
            // The words, in any case, with a sign.
            "inf",
            "INF",
            "-inf",
            "+inf",
            "infinity",
            "Infinity",
            "-INFINITY",
            "nan",
            "NaN",
            "-nan",
            "+NAN",
            // Not the grammar: each is 0.0.
            "",
            "+",
            "-",
            ".",
            "e5",
            ".e1",
            "1e",
            "1e+",
            "1.5.2",
            "1e5.5",
            "--1",
            "+-1",
            " 1",
            "1 ",
            "1_000",
            "0x1A",
            "0b1",
            "infinit",
            "infinityy",
            "nanx",
            "in",
            "∞",
            "١٢",
            "1,5",
        ];
        let literal = |s: &str| {
            let escaped: String = s
                .chars()
                .map(|c| format!("\\u{{{:x}}}", u32::from(c)))
                .collect();
            format!("\"{escaped}\"")
        };
        let list: Vec<String> = cases.iter().map(|s| literal(s)).collect();
        let script = format!(
            "{FLOAT}\nconst view = new DataView(new ArrayBuffer(8));\n\
             for (const s of [{}]) {{\n  const v = floatFrom(s);\n  view.setFloat64(0, v);\n  \
             console.log(Number.isNaN(v) ? \"NaN\" : view.getBigUint64(0).toString(16));\n}}\n",
            list.join(", ")
        );
        let dir = std::env::temp_dir().join("nova-codegen-wasm-float-from");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        let file = dir.join("float_from.js");
        std::fs::write(&file, script).expect("the script");
        let out = node.arg(&file).output().expect("node runs");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let got = String::from_utf8(out.stdout).expect("UTF-8");
        let got: Vec<&str> = got.lines().collect();
        assert_eq!(got.len(), cases.len(), "{got:?}");
        for (case, got) in cases.iter().zip(got) {
            let v = case.parse::<f64>().unwrap_or(0.0);
            let expected = if v.is_nan() {
                "NaN".to_string()
            } else {
                format!("{:x}", v.to_bits())
            };
            assert_eq!(got, expected, "floatFrom({case:?})");
        }
    }
}
