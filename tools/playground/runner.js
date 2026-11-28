// The playground's runner (spec
// docs/superpowers/specs/2026-11-28-phase-6-1-the-playground-design.md
// §3.6): the script of each Run's sandboxed iframe, whose `srcdoc` the page
// writes with this text inline, allowed by a per-Run nonce in the frame's
// own policy and by its hash in the page's. The frame's origin is opaque.
//
// It posts `{ kind: "ready" }` to the page, takes one `{ js, wasm, page }`
// from it, and loads `app.js` from a `blob:` URL, the one way Chromium
// runs a module the frame was given as text under its policy. A program
// that draws (`page`) runs here, on the frame's thread, into
// `<div id="app">`. A console program runs in a worker the runner starts,
// so that a program that never ends leaves the frame, and the page, free
// to answer: a classic worker, which imports `app.js` itself, since
// Chromium does not start a module worker from a `blob:` URL in an opaque
// origin. Each write goes to the page as `{ kind: "write", fd, text }`,
// then `{ kind: "end" }` when `init` resolves or `{ kind: "error", text }`
// if it rejects. The page stops a program by removing the frame, and the
// worker with it.
//
// No closing script tag, nor the opening of an HTML comment, may appear in
// this file, which the page puts inside a script element; `build.mjs`
// checks.

(() => {
  "use strict";

  const post = (message) => parent.postMessage(message, "*");

  // Run `app.js`, given as text, with `wasm`, sending each write to
  // `send`; `send` gets the end or the error too.
  async function run(js, wasm, send) {
    const url = URL.createObjectURL(new Blob([js], { type: "text/javascript" }));
    try {
      const { default: init } = await import(url);
      await init({ wasm, print: (fd, text) => send({ kind: "write", fd, text }) });
      send({ kind: "end" });
    } catch (error) {
      send({ kind: "error", text: String(error) });
    } finally {
      URL.revokeObjectURL(url);
    }
  }

  // A console program's worker: its source is `run` and this function's,
  // as text.
  function programWorker() {
    onmessage = (event) => {
      onmessage = null;
      run(event.data.js, event.data.wasm, (message) => postMessage(message));
    };
  }

  let given = false;
  addEventListener("message", (event) => {
    if (event.source !== parent || given) return;
    given = true;
    const { js, wasm, page } = event.data;
    if (page) {
      run(js, wasm, post);
      return;
    }
    const source = `"use strict";\n${run}\n(${programWorker})();\n`;
    const worker = new Worker(URL.createObjectURL(new Blob([source], { type: "text/javascript" })));
    worker.onmessage = (message) => post(message.data);
    worker.onerror = (error) => post({ kind: "error", text: error.message || "the program's worker failed to start" });
    worker.postMessage({ js, wasm }, [wasm.buffer]);
  });

  post({ kind: "ready" });
})();
