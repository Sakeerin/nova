// The playground's compiler worker (spec
// docs/superpowers/specs/2026-11-28-phase-6-1-the-playground-design.md
// §3.6): a module worker of the page's origin. It fetches
// `nova_playground.wasm` beside itself, compiles it once through
// `compiler.js`, and posts `{ kind: "ready", ms }`, the time that took (or
// `{ kind: "load-error", text }`); then it answers each `{ id, source }`
// with the compile's result and the same `id`, its `kind` one of
// `compiled` (`wasm`, `js`, `page`, `warnings`), `failed` (`text`,
// `errors`), `too-long` or `internal` (`text`), each with `ms`.
// A compile that never ends is the page's to stop: it replaces the worker.

import { Compiler } from "./compiler.js";

const started = performance.now();
const loading = (async () => {
  const response = await fetch(new URL("nova_playground.wasm", import.meta.url));
  if (!response.ok) throw new Error(`nova_playground.wasm: ${response.status} ${response.statusText}`);
  return Compiler.load(await response.arrayBuffer());
})();

loading.then(
  () => postMessage({ kind: "ready", ms: performance.now() - started }),
  (error) => postMessage({ kind: "load-error", text: String(error) }),
);

onmessage = async (event) => {
  const { id, source } = event.data;
  const compiler = await loading;
  const result = await compiler.compile(source);
  const transfer = result.kind === "compiled" ? [result.wasm.buffer] : [];
  postMessage({ id, ...result }, transfer);
};
