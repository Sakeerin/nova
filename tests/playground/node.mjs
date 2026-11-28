// The playground's compiler under Node (spec
// docs/superpowers/specs/2026-11-28-phase-6-1-the-playground-design.md
// §5, test 6), run by CI's Playground job on the page `build.mjs` built:
//
//     node tests/playground/node.mjs <page>
//
// It loads the shipped module, `<page>/nova_playground.wasm`, through the
// page's own `compiler.js`, and checks: a fresh instance's memory is over
// 4 MiB (the shadow stack `build.rs` sets); each console example of
// `<page>/examples.json` compiles and, run by `init({ wasm, print })` from
// a `data:` import, prints its expected text, with nothing on standard
// error; `page` is set for the counter alone; type_error's diagnostics and
// error count; the depth cases of `depth.mjs` at 128 (no throw, each typed
// kind's output, nested `fn` items E0900) and 129 (P0002); 10,000 `if`s in
// a row are `too-long`, and hello world compiles on the next instance; and
// 10,000 `while` loops meet the locals limit. It prints the figures. It
// reads no file outside `<page>` and this directory, and writes none.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { parseKinds, typedKinds } from "./depth.mjs";

const started = performance.now();
if (process.argv.length !== 3) throw new Error("usage: node tests/playground/node.mjs <page>");
const page = resolve(process.argv[2]);
const { Compiler } = await import(pathToFileURL(join(page, "compiler.js")).href);
const examples = JSON.parse(readFileSync(join(page, "examples.json"), "utf8"));
const example = (name) => examples.find((e) => e.name === name);

const loadStart = performance.now();
const compiler = await Compiler.load(readFileSync(join(page, "nova_playground.wasm")));
const loadMs = performance.now() - loadStart;
const fresh = compiler.memoryBytes();
assert.ok(fresh > 4 << 20, `a fresh instance's memory is ${fresh} bytes, not over 4 MiB`);

// Run a compiled program as the page's runner does: `app.js` imported from
// its text, a fresh module each time, every write taken by `print`.
let runs = 0;
async function run({ js, wasm }) {
  runs += 1;
  const text = `${js}\n// run ${runs}\n`;
  const { default: init } = await import(`data:text/javascript;base64,${Buffer.from(text).toString("base64")}`);
  const out = ["", "", ""];
  await init({ wasm, print: (fd, s) => (out[fd] += s) });
  return { stdout: out[1], stderr: out[2] };
}

const codes = (text) => [...new Set(Array.from(text.matchAll(/error\[([A-Z]\d{4})\]/g), (m) => m[1]))];
const ms = (x) => `${x.toFixed(1)} ms`;

// The fixed list.
const figures = [];
for (const { name, source, expected } of examples) {
  const r = await compiler.compile(source);
  figures.push(`${name} ${ms(r.ms)}`);
  if (name === "type_error") {
    assert.equal(r.kind, "failed", `type_error: ${r.kind}`);
    assert.equal(r.errors, 1, "type_error's error count");
    assert.equal(r.text, expected, "type_error's diagnostics");
    continue;
  }
  assert.equal(r.kind, "compiled", `${name}: ${r.kind} ${r.text ?? ""}`);
  assert.equal(r.page, name === "counter", `${name}: page is ${r.page}`);
  if (r.page) continue;
  const { stdout, stderr } = await run(r);
  assert.equal(stdout, expected, `${name}'s output`);
  assert.equal(stderr, "", `${name}'s standard error`);
}

// The depth cases: no throw at 128, P0002 at 129.
let ran = 0;
for (const [kind, make] of typedKinds) {
  const at128 = make(128);
  const r = await compiler.compile(at128.source);
  if (at128.code !== undefined) {
    assert.equal(r.kind, "failed", `typed ${kind} at 128: ${r.kind}`);
    assert.deepEqual(codes(r.text), [at128.code], `typed ${kind} at 128: ${r.text}`);
  } else {
    assert.equal(r.kind, "compiled", `typed ${kind} at 128: ${r.kind} ${r.text ?? ""}`);
    assert.deepEqual(await run(r), { stdout: at128.prints, stderr: "" }, `typed ${kind} at 128`);
    ran += 1;
  }
}
for (const [kind, make] of parseKinds) {
  const r = await compiler.compile(make(128));
  assert.ok(r.kind === "failed" || r.kind === "compiled", `5.1b's ${kind} at 128: ${r.kind} ${r.text ?? ""}`);
  if (r.kind === "failed") assert.ok(!codes(r.text).includes("P0002"), `5.1b's ${kind} at 128: ${r.text}`);
}
const kinds = [...typedKinds.map(([k, make]) => [`typed ${k}`, (d) => make(d).source]), ...parseKinds];
for (const [kind, make] of kinds) {
  const r = await compiler.compile(make(129));
  assert.equal(r.kind, "failed", `${kind} at 129: ${r.kind}`);
  assert.ok(codes(r.text).includes("P0002"), `${kind} at 129: ${r.text}`);
}

// Long function bodies: the compiler's stack, then the locals limit.
const ifs = (n) =>
  `fn main() {\n    let x = 3\n    let mut y = 0\n${Array.from({ length: n }, (_, k) => `    if x > ${k % 7} { y = y + 1 }\n`).join("")}    println("\${y}")\n}\n`;
const whiles = (n) =>
  `fn main() {\n    let mut y = 0\n${Array.from({ length: n }, () => "    let mut i = 0\n    while i < 1 {\n        i = i + 1\n        y = y + 1\n    }\n").join("")}    println("\${y}")\n}\n`;
const long = await compiler.compile(ifs(10_000));
assert.equal(long.kind, "too-long", `10,000 ifs: ${long.kind} ${long.text ?? ""}`);
const after = await compiler.compile(example("hello").source);
assert.equal(after.kind, "compiled", `hello after the overflow: ${after.kind}`);
assert.deepEqual(await run(after), { stdout: "Hello, World!\n", stderr: "" }, "hello after the overflow");
const locals = await compiler.compile(whiles(10_000));
assert.equal(locals.kind, "failed", `10,000 whiles: ${locals.kind}`);
assert.equal(locals.errors, 1, "10,000 whiles' error count");
assert.match(locals.text, /^Error: function `main` needs \d+ locals, but WebAssembly allows 50000\n$/m);

console.log(
  `playground (Node ${process.version}): loaded in ${ms(loadMs)}, fresh memory ${fresh} bytes; ` +
    `compiles: ${figures.join(", ")}; depth: ${typedKinds.length + parseKinds.length} kinds, ${ran} ran right at 128, ` +
    `all P0002 at 129; 10,000 ifs too long in ${ms(long.ms)}, then hello; 10,000 whiles at the locals limit; ` +
    `${ms(performance.now() - started)} in all`,
);
