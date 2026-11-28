// Build the playground's page (spec
// docs/superpowers/specs/2026-11-28-phase-6-1-the-playground-design.md
// §3.6), on Node's built-in modules alone:
//
//     node tools/playground/build.mjs <compiler.wasm> <out> [--wasm-opt]
//
// `<compiler.wasm>` is `nova-playground` built for wasm32-unknown-unknown
// with `--profile playground`. The build copies the page's files into
// `<out>`, the module as `nova_playground.wasm` (through binaryen's
// `wasm-opt`, from `PATH`, with `--wasm-opt`), writes the runner's hash
// into `index.html`'s policy, and writes `examples.json` from
// `examples.txt`, reading each text where it lives, so the sources are
// kept once. It prints the module's sizes, raw, optimised and gzipped
// (zlib's level 9, as `tests/browser/spa.mjs` measures), and fails at
// 5,000,000 bytes gzipped (B12). Every URL in the page is relative.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";

const LIMIT = 5_000_000;
// Stable's wasm32 features beyond binaryen's defaults, which binaryen
// refuses in its input unless they are enabled (as
// `crates/nova-cli/src/bundle.rs` enables bulk memory for `nova`'s own).
const WASM_OPT = [
  "-Oz",
  "--enable-bulk-memory",
  "--enable-multivalue",
  "--enable-nontrapping-float-to-int",
  "--enable-reference-types",
];
const FILES = ["index.html", "style.css", "page.js", "compiler.js", "worker.js", "runner.js"];

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..", "..");

const args = process.argv.slice(2);
const wasmOpt = args.includes("--wasm-opt");
const paths = args.filter((a) => a !== "--wasm-opt");
if (paths.length !== 2 || args.length - paths.length > 1) {
  console.error("usage: node tools/playground/build.mjs <compiler.wasm> <out> [--wasm-opt]");
  process.exit(2);
}
const [input, out] = paths.map((p) => resolve(p));
mkdirSync(out, { recursive: true });

// The module.
const raw = readFileSync(input);
const shipped = join(out, "nova_playground.wasm");
if (wasmOpt) {
  execFileSync("wasm-opt", [...WASM_OPT, input, "-o", shipped], { stdio: "inherit" });
} else {
  writeFileSync(shipped, raw);
}
const module = readFileSync(shipped);
const gzipped = gzipSync(module, { level: 9 }).length;

// The runner goes inside a script element of each Run's `srcdoc`, so it
// may not close it or open an HTML comment; the HTML parser turns CRLF
// into LF there, so its hash is taken over LF line ends, as it is copied.
const runner = readFileSync(join(here, "runner.js"), "utf8").replace(/\r\n/g, "\n");
if (/<\/script|<!--/i.test(runner)) throw new Error("runner.js may not hold a closing script tag or `<!--`");
const hash = createHash("sha256").update(runner, "utf8").digest("base64");

for (const name of FILES) {
  let text = readFileSync(join(here, name), "utf8");
  if (name === "runner.js") text = runner;
  if (name === "index.html") {
    if (text.split("sha256-RUNNER_HASH").length !== 2) throw new Error("index.html: no single sha256-RUNNER_HASH");
    text = text.replace("sha256-RUNNER_HASH", `sha256-${hash}`);
  }
  writeFileSync(join(out, name), text);
}

// examples.json, from the fixed list: `name | label | source | expected`,
// each path from the repository's root, `-` for no expected text.
const examples = [];
for (const line of readFileSync(join(here, "examples.txt"), "utf8").split(/\r?\n/)) {
  if (line.trim() === "" || line.startsWith("#")) continue;
  const fields = line.split(" | ");
  if (fields.length !== 4) throw new Error(`examples.txt: not four fields: ${line}`);
  const [name, label, source, expected] = fields.map((f) => f.trim());
  examples.push({
    name,
    label,
    source: readFileSync(join(root, source), "utf8"),
    expected: expected === "-" ? null : readFileSync(join(root, expected), "utf8"),
  });
}
writeFileSync(join(out, "examples.json"), `${JSON.stringify(examples, null, 1)}\n`);

const n = (x) => x.toLocaleString("en-US");
console.log(`the playground's page in ${out}: ${FILES.length + 2} files, ${examples.length} examples`);
console.log(
  `nova_playground.wasm: ${n(raw.length)} bytes raw; ` +
    (wasmOpt ? `${n(module.length)} bytes after wasm-opt ${WASM_OPT.join(" ")}; ` : "wasm-opt not run; ") +
    `${n(gzipped)} bytes gzipped (zlib level 9), the limit ${n(LIMIT)}`,
);
if (gzipped >= LIMIT) {
  console.error(`nova_playground.wasm is ${n(gzipped)} bytes gzipped, at or over the limit of ${n(LIMIT)}`);
  process.exit(1);
}
