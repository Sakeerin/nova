// The playground's compiler driver (spec
// docs/superpowers/specs/2026-11-28-phase-6-1-the-playground-design.md
// §3.6, §3.7), shared by the page's worker (`worker.js`) and the Node test
// (`tests/playground/node.mjs`): it compiles the module `nova-playground`
// once, instantiates it, and calls its C ABI (`crates/nova-playground/src/abi.rs`):
// `alloc` and `dealloc` for the source's bytes, `compile`, which returns
// nine words, and `panic_info`, two words a panic hook fills.
//
// A throw out of `compile` leaves the instance past use. It is classified
// (§3.7): a `RangeError` about the call stack, or an out-of-bounds trap
// with no panic message, is the compiler running out of stack on a long
// function body, `too-long`; anything else is `internal`. Either way the
// next compile runs on a fresh instance of the module already compiled.

const encoder = new TextEncoder();
const decoder = new TextDecoder();

// The words `compile` returns: the status, then `app.wasm`'s, `app.js`'s
// and the text's address and length, the error count, and `page`.
const STATUS = 0;
const WASM = 1;
const JS = 3;
const TEXT = 5;
const ERRORS = 7;
const PAGE = 8;

// Whether a throw out of `compile`, with the panic message `panic` ("" if
// none), is the stack running out on a long function body (§3.7): V8's own
// stack, or Rust's shadow stack, which ends in an out-of-bounds access.
export function tooLong(error, panic) {
  if (panic !== "") return false;
  if (error instanceof RangeError) return /call stack/i.test(error.message);
  if (error instanceof WebAssembly.RuntimeError) return /memory access out of bounds/i.test(error.message);
  return false;
}

export class Compiler {
  // The compiler from the module's bytes, compiled once, whatever the
  // MIME type they were served with.
  static async load(bytes) {
    const module = await WebAssembly.compile(bytes);
    const compiler = new Compiler(module);
    await compiler.instantiate();
    return compiler;
  }

  constructor(module) {
    this.module = module;
    this.exports = null;
    this.panicAt = 0;
  }

  // A fresh instance of the module; the address of `panic_info`'s two
  // words is taken now, since a trapped instance is never called again.
  async instantiate() {
    const instance = await WebAssembly.instantiate(this.module, {});
    this.exports = instance.exports;
    this.panicAt = this.exports.panic_info();
  }

  // The instance's linear memory, in bytes.
  memoryBytes() {
    return this.exports.memory.buffer.byteLength;
  }

  // The last panic's message, read from memory: "" if there was none.
  panicMessage() {
    const { buffer } = this.exports.memory;
    const [at, len] = new Uint32Array(buffer, this.panicAt, 2);
    return len === 0 ? "" : decoder.decode(new Uint8Array(buffer, at, len));
  }

  // Compile `source`: `{ kind: "compiled", wasm, js, page, warnings }`,
  // `{ kind: "failed", text, errors }`, `{ kind: "too-long" }` or
  // `{ kind: "internal", text }`, each with `ms`, the compile's time.
  async compile(source) {
    const ex = this.exports;
    const bytes = encoder.encode(source);
    const at = ex.alloc(bytes.length);
    new Uint8Array(ex.memory.buffer, at, bytes.length).set(bytes);
    const start = performance.now();
    let words;
    try {
      words = ex.compile(at, bytes.length);
    } catch (error) {
      const ms = performance.now() - start;
      const panic = this.panicMessage();
      await this.instantiate();
      if (tooLong(error, panic)) return { kind: "too-long", ms };
      return { kind: "internal", text: panic !== "" ? panic : `${error.name}: ${error.message}`, ms };
    }
    const ms = performance.now() - start;
    ex.dealloc(at, bytes.length);
    const { buffer } = ex.memory;
    const w = new Uint32Array(buffer, words, 9);
    // Copies: the buffers live only until the next compile.
    const bytesAt = (i) => new Uint8Array(buffer, w[i], w[i + 1]).slice();
    const textAt = (i) => decoder.decode(new Uint8Array(buffer, w[i], w[i + 1]));
    switch (w[STATUS]) {
      case 0:
        return { kind: "compiled", wasm: bytesAt(WASM), js: textAt(JS), page: w[PAGE] === 1, warnings: textAt(TEXT), ms };
      case 1:
        return { kind: "failed", text: textAt(TEXT), errors: w[ERRORS], ms };
      default:
        return { kind: "internal", text: textAt(TEXT), ms };
    }
  }
}
