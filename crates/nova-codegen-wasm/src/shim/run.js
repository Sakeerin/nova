
async function load(options) {
  const imports = { env };
  if (options.wasm) return WebAssembly.instantiate(options.wasm, imports);
  const url = new URL("app.wasm", import.meta.url);
  if (isNode) {
    const { readFile } = await import("node:fs/promises");
    return WebAssembly.instantiate(await readFile(url), imports);
  }
  return WebAssembly.instantiateStreaming(fetch(url), imports);
}

// Load app.wasm and run `main`. A panic or a trap ends the program as a
// native build ends: its message on stderr and, under Node, the exit status
// of an abort (134) or of a trap (132).
export default async function init(options = {}) {
  const { instance } = await load(options);
  wasm = instance.exports;
  try {
    wasm.main();
  } catch (e) {
    let status;
    if (e instanceof NovaPanic) {
      status = 134;
    } else if (e instanceof RangeError && /call stack/i.test(e.message)) {
      write(2, "nova: stack overflow\n");
      status = 134;
    } else if (e instanceof WebAssembly.RuntimeError) {
      write(2, `nova: trap: ${e.message}\n`);
      status = 132;
    } else {
      throw e;
    }
    if (isNode) process.exitCode = status;
  } finally {
    flush();
  }
}

// Node resolves symbolic links in this module's URL, so the path it was run
// by is resolved the same way before the two are compared.
if (isNode && process.argv[1]) {
  const { pathToFileURL } = await import("node:url");
  const { realpathSync } = await import("node:fs");
  if (import.meta.url === pathToFileURL(realpathSync(process.argv[1])).href) await init();
}
