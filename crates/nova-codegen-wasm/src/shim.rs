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
const DOM: &str = include_str!("shim/dom.js");
const DOM_CALL: &str = include_str!("shim/dom_call.js");
const DOM_FETCH: &str = include_str!("shim/dom_fetch.js");

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

/// The two clocks (spec 4.1 §7.1). The monotonic one counts from its first
/// reading, as `nova-runtime`'s does, and every step of it is
/// non-decreasing; the wall clock is 0 before 1970 and saturates at
/// `i64::MAX`.
const CLOCKS: &str = "
// The monotonic clock, in nanoseconds since its first reading.
let origin;
function nowNanos() {
  const t = performance.now();
  if (origin === undefined) origin = t;
  return BigInt(Math.floor((t - origin) * 1e6));
}

// The wall clock, in nanoseconds since 1970, to the millisecond.
function epochNanos() {
  const ms = Date.now();
  if (ms < 0) return 0n;
  const ns = BigInt(ms) * 1000000n;
  return ns > 9223372036854775807n ? 9223372036854775807n : ns;
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
        "time_now_nanos" => "  time_now_nanos: () => nowNanos(),\n",
        "time_now_epoch_nanos" => "  time_now_epoch_nanos: () => epochNanos(),\n",
        "abort" => "  abort: () => {\n    throw new NovaPanic();\n  },\n",
        other => bail!("the shim has no host function `{other}`"),
    })
}

/// The JS of std's `extern "js"` function `name` (spec
/// `docs/superpowers/specs/2026-10-16-phase-4-2-std-dom-and-the-shim-design.md`
/// §3.1, §6.2), or `None` for a name std does not declare. Handles are
/// `BigInt`s, strings addresses, and a `Bool` 0 or 1.
fn js_function(name: &str) -> Option<&'static str> {
    Some(match name {
        "dom_body" => "() => handle(document.body)",
        "dom_create_element" => "(tag) => handle(document.createElement(text(tag)))",
        "dom_create_text" => "(s) => handle(document.createTextNode(text(s)))",
        "dom_query_selector" => "(sel) => handle(document.querySelector(text(sel)))",
        "dom_query_selector_in" => "(el, sel) => handle(object(el).querySelector(text(sel)))",
        "dom_query_all" => "(sel) => handle(Array.from(document.querySelectorAll(text(sel))))",
        "dom_list_len" => "(list) => BigInt(object(list).length)",
        "dom_list_item" => "(list, i) => handle(object(list)[Number(i)])",
        "dom_element_by_id" => "(id) => handle(document.getElementById(text(id)))",
        "dom_tag" => "(el) => string(object(el).nodeName.toLowerCase())",
        "dom_set_attribute" => "(el, n, v) => { object(el).setAttribute(text(n), text(v)); }",
        "dom_has_attribute" => "(el, n) => (object(el).hasAttribute(text(n)) ? 1 : 0)",
        "dom_attribute" => "(el, n) => string(object(el).getAttribute(text(n)) ?? \"\")",
        "dom_remove_attribute" => "(el, n) => { object(el).removeAttribute(text(n)); }",
        "dom_set_property" => "(el, n, v) => { object(el)[text(n)] = text(v); }",
        "dom_property" => "(el, n) => string(String(object(el)[text(n)] ?? \"\"))",
        "dom_set_property_bool" => "(el, n, v) => { object(el)[text(n)] = v !== 0; }",
        "dom_property_bool" => "(el, n) => (object(el)[text(n)] ? 1 : 0)",
        "dom_set_text" => "(el, s) => { object(el).textContent = text(s); }",
        "dom_text" => "(el) => string(object(el).textContent ?? \"\")",
        "dom_append_child" => "(p, c) => { object(p).appendChild(object(c)); }",
        "dom_insert_before" => {
            "(p, c, b) => { object(p).insertBefore(object(c), b === 0n ? null : object(b)); }"
        }
        "dom_remove_child" => "(p, c) => { object(p).removeChild(object(c)); }",
        "dom_remove" => "(el) => { object(el).remove(); }",
        "dom_parent" => "(el) => handle(object(el).parentNode)",
        "dom_first_child" => "(el) => handle(object(el).firstChild)",
        "dom_next_sibling" => "(el) => handle(object(el).nextSibling)",
        "dom_is_same" => "(a, b) => (object(a) === object(b) ? 1 : 0)",
        "host_release" => "(id) => { release(id); }",
        "host_handles" => "() => BigInt(handles.length - 1 - freeHandles.length)",
        "dom_listen" => {
            "(t, kind, id) => { const f = listener(id); object(t).addEventListener(text(kind), f); return handle(f); }"
        }
        "dom_unlisten" => {
            "(t, kind, f) => { object(t).removeEventListener(text(kind), object(f)); }"
        }
        "dom_window_listen" => {
            "(kind, id) => { const f = listener(id); window.addEventListener(text(kind), f); return handle(f); }"
        }
        "dom_window_unlisten" => "(kind, f) => { window.removeEventListener(text(kind), object(f)); }",
        "dom_event_kind" => "(ev) => string(object(ev).type)",
        "dom_event_target" => {
            "(ev) => { const t = object(ev).target; return t === window ? 0n : handle(t); }"
        }
        "dom_event_key" => "(ev) => string(object(ev).key ?? \"\")",
        "dom_prevent_default" => "(ev) => { object(ev).preventDefault(); }",
        "dom_stop_propagation" => "(ev) => { object(ev).stopPropagation(); }",
        "dom_click" => "(el) => { object(el).click(); }",
        // A delay JS cannot hold, over 2^31 - 1 ms, would fire at once; an
        // interval clears itself once the program has ended, so Node exits.
        "dom_set_timeout" => {
            "(ms, id) => handle(setTimeout(() => call(id, 0n), Math.min(Number(ms), 2147483647)))"
        }
        "dom_set_interval" => {
            "(ms, id) => { const t = setInterval(() => { if (ended) clearInterval(t); else call(id, 0n); }, Math.min(Number(ms), 2147483647)); return handle(t); }"
        }
        "dom_clear_timeout" => "(t) => { clearTimeout(object(t)); }",
        "dom_clear_interval" => "(t) => { clearInterval(object(t)); }",
        "dom_location_href" => "() => string(location.href)",
        "dom_location_pathname" => "() => string(location.pathname)",
        "dom_location_search" => "() => string(location.search)",
        "dom_location_hash" => "() => string(location.hash)",
        "dom_set_location_hash" => "(h) => { location.hash = text(h); }",
        "dom_history_push" => "(url) => { history.pushState(null, \"\", text(url)); }",
        "dom_history_replace" => "(url) => { history.replaceState(null, \"\", text(url)); }",
        "dom_history_back" => "() => { history.back(); }",
        "dom_fetch" => "(url, method, headers, body) => fetchToken(text(url), text(method), text(headers), text(body))",
        "host_done" => "(t) => (object(t).done ? 1 : 0)",
        "dom_fetch_failed" => "(t) => (object(t).error !== undefined ? 1 : 0)",
        "dom_fetch_error" => "(t) => string(object(t).error)",
        "dom_fetch_status" => "(t) => BigInt(object(t).status)",
        "dom_fetch_ok" => "(t) => (object(t).ok ? 1 : 0)",
        "dom_fetch_body" => "(t) => string(object(t).body)",
        _ => return None,
    })
}

/// An `env` entry for `name`: a host function of the runtime's, or std's
/// `extern "js"` function, guarded (spec 4.2 §3.1).
fn entry(name: &str) -> Result<String> {
    if let Some(f) = js_function(name) {
        return Ok(format!("  {name}: guard(\"{name}\", {f}),\n"));
    }
    host(name).map(str::to_string)
}

/// `app.js` for a module importing `imports` from `env`.
pub fn shim<S: AsRef<str>>(imports: &[S]) -> Result<String> {
    let imports: Vec<&str> = imports.iter().map(AsRef::as_ref).collect();
    let mut js = String::from(CORE);
    let uses = |names: &[&str]| imports.iter().any(|i| names.contains(i));
    if uses(&["float_to_str", "float_fixed", "str_to_float"]) {
        js.push_str(FLOAT);
    }
    if uses(&["int_hash_seed"]) {
        js.push_str(SEED);
    }
    if uses(&["time_now_nanos", "time_now_epoch_nanos"]) {
        js.push_str(CLOCKS);
    }
    if imports.iter().any(|i| js_function(i).is_some()) {
        js.push_str(DOM);
    }
    if uses(&[
        "dom_listen",
        "dom_window_listen",
        "dom_set_timeout",
        "dom_set_interval",
    ]) {
        js.push_str(DOM_CALL);
    }
    if uses(&["dom_fetch"]) {
        js.push_str(DOM_FETCH);
    }
    js.push_str("\n// The host functions the module imports.\nconst env = {\n");
    for name in &imports {
        js.push_str(&entry(name)?);
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
        assert!(!plain.contains("function nowNanos"), "{plain}");
        let floats = shim(&["float_fixed", "int_hash_seed"]).expect("a shim");
        assert!(floats.contains("function floatFixed"), "{floats}");
        assert!(floats.contains("function hashSeed"), "{floats}");
        let clocks = shim(&["time_now_epoch_nanos"]).expect("a shim");
        assert!(clocks.contains("function epochNanos"), "{clocks}");
    }

    /// The monotonic clock starts at 0 and never decreases over a busy
    /// loop; the wall clock is within a second of `Date.now()` (spec 4.1
    /// §9.4).
    #[test]
    fn the_clocks_are_monotonic_and_the_wall_clock_is_now() {
        let Some(mut node) = crate::test_node() else {
            return;
        };
        let script = format!(
            "{CLOCKS}\nconst first = nowNanos();\nlet last = first;\n\
             for (let i = 0; i < 100000; i++) {{\n  const t = nowNanos();\n  \
             if (t < last) throw new Error(`went back: ${{last}} then ${{t}}`);\n  last = t;\n}}\n\
             const wall = epochNanos() - BigInt(Date.now()) * 1000000n;\n\
             console.log(first, last > 0n, wall >= -1000000000n && wall <= 1000000000n);\n"
        );
        let dir = std::env::temp_dir().join("nova-codegen-wasm-clocks");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        let file = dir.join("clocks.js");
        std::fs::write(&file, script).expect("the script");
        let out = node.arg(&file).output().expect("node runs");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&out.stdout), "0n true true\n");
    }

    /// std's `extern "js"` functions come guarded, with the handle table,
    /// and only the imported ones (spec 4.2 §3.1, §8).
    #[test]
    fn js_functions_come_guarded_and_only_when_imported() {
        let plain = shim(&["println"]).expect("a shim");
        assert!(!plain.contains("function handle("), "{plain}");
        let dom = shim(&["println", "dom_create_element"]).expect("a shim");
        assert!(
            dom.contains("  dom_create_element: guard(\"dom_create_element\", (tag) =>"),
            "{dom}"
        );
        assert!(dom.contains("function handle("), "{dom}");
        assert!(!dom.contains("dom_set_text"), "{dom}");
    }

    /// A JS exception in a std/dom function is a Nova panic that names the
    /// function and quotes the exception; a handle the table does not hold
    /// is a panic too (spec 4.2 §3.1, §4.1).
    #[test]
    fn a_js_exception_is_a_panic_naming_its_function() {
        let Some(mut node) = crate::test_node() else {
            return;
        };
        let js = shim(&["dom_create_element", "dom_tag"]).expect("a shim");
        let core = js
            .split("\n// The host functions the module imports.")
            .next()
            .unwrap();
        let script = format!(
            "{core}\nlet ended = false;\nconst lines = [];\nwrite = (fd, s) => lines.push(s);\n\
             globalThis.document = {{ createElement: () => {{ const e = new Error(\"boom\"); e.name = \"NotSupportedError\"; throw e; }} }};\n\
             text = (s) => s;\n\
             const create = guard(\"dom_create_element\", (tag) => handle(document.createElement(text(tag))));\n\
             for (const f of [() => create(\"div\"), () => object(7n)]) {{\n  \
             try {{ f(); }} catch (e) {{ lines.push(e instanceof NovaPanic ? \"[panic]\" : String(e)); }}\n}}\n\
             console.log(lines.join(\"\"));\n"
        );
        let dir = std::env::temp_dir().join("nova-codegen-wasm-guard");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        let file = dir.join("guard.mjs");
        std::fs::write(&file, script).expect("the script");
        let out = node.arg(&file).output().expect("node runs");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "nova: panic: std/dom: dom_create_element: NotSupportedError: boom\n[panic]\
             nova: panic: std/dom: handle 7 is not live\n[panic]\n"
        );
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
