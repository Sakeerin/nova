// The playground's page (spec
// docs/superpowers/specs/2026-11-28-phase-6-1-the-playground-design.md
// §3.6, §3.7): the compiler in a worker (`worker.js`), each Run's program
// in a fresh sandboxed iframe (`runner.js`), the limits, Stop, and the
// page's messages. Nothing is stored, and nothing is read from the URL.
//
// The status line carries what the tests read: `data-state` (loading,
// ready, compiling, running, waiting, done, failed, too-long, internal,
// stopped, error), `data-run` (the Runs so far), `data-ready-ms` (when
// the compiler was first ready, in milliseconds since navigation) and
// `data-compile-ms` (the last compile's time in the worker).

const RUN_LIMIT_MS = 10_000;
const COMPILE_LIMIT_MS = 30_000;

const MESSAGES = {
  internal:
    "The compiler failed on this program: an internal error, a bug in Nova. Please report it with the program. The compiler said:",
  runLimit: "Stopped: the program was still running after 10 seconds.",
  stopped: "Stopped.",
  compileLimit: "The compiler took more than 30 seconds and was restarted.",
};

const $ = (id) => document.getElementById(id);
const editor = $("editor");
const examples = $("examples");
const runButton = $("run");
const stopButton = $("stop");
const status = $("status");
const output = $("output");
const frameBox = $("frame");

let runner = "";
let worker = null;
let ready = false;
// The Runs so far; each Run's number is its compile's id.
let runs = 0;
// The compile in flight: its id and its time limit.
let compiling = null;
// The program's frame, whether it draws there, what it is to run, and its
// time limit.
let frame = null;
let drawing = false;
let program = null;
let runTimer = null;

// The compiler is ready, the runner's text is here, and no compile is in
// flight.
function canRun() {
  return ready && runner !== "" && compiling === null;
}

function setStatus(state, text) {
  status.dataset.state = state;
  status.textContent = text;
  runButton.disabled = !canRun();
  stopButton.disabled = compiling === null && frame === null;
}

// A piece of the output pane, as text: `out` and `err` the program's
// standard output and error, `diag` diagnostics, `note` the page's own.
function show(kind, text) {
  const span = document.createElement("span");
  span.className = kind;
  span.textContent = text;
  output.append(span);
  return span;
}

function note(text) {
  show("note", `${text}\n`);
}

function tooLong() {
  const span = show("note", "");
  const code = document.createElement("code");
  code.textContent = "nova build --target wasm";
  span.append(
    "The playground's compiler ran out of stack on this program: one of its functions is probably too long for the compiler in the browser. Splitting it into smaller functions should help, and ",
    code,
    " compiles it as it is on your own machine.\n",
  );
}

// The compiler's worker, started afresh after a compile past its limit or
// stopped.
function startWorker() {
  ready = false;
  const w = new Worker("worker.js", { type: "module" });
  w.onmessage = ({ data }) => {
    if (w !== worker) return;
    if (data.kind === "ready") {
      ready = true;
      if (status.dataset.readyMs === undefined) status.dataset.readyMs = performance.now().toFixed(1);
      if (status.dataset.state === "loading") setStatus("ready", `Ready: the compiler loaded in ${Math.round(data.ms)} ms.`);
      else setStatus(status.dataset.state, status.textContent);
    } else if (data.kind === "load-error") {
      setStatus("error", `The compiler could not be loaded: ${data.text}`);
    } else if (compiling !== null && data.id === compiling.id) {
      clearTimeout(compiling.timer);
      compiling = null;
      compiled(data);
    }
  };
  w.onerror = (event) => {
    if (w === worker) setStatus("error", `The compiler could not be loaded: ${event.message ?? "the worker failed"}`);
  };
  worker = w;
}

function replaceWorker() {
  clearTimeout(compiling.timer);
  compiling = null;
  worker.terminate();
  startWorker();
}

function run() {
  if (!canRun()) return;
  stopProgram();
  output.replaceChildren();
  runs += 1;
  status.dataset.run = String(runs);
  const id = runs;
  compiling = {
    id,
    timer: setTimeout(() => {
      replaceWorker();
      note(MESSAGES.compileLimit);
      setStatus("stopped", "The compiler was restarted.");
    }, COMPILE_LIMIT_MS),
  };
  setStatus("compiling", "Compiling…");
  worker.postMessage({ id, source: editor.value });
}

function compiled(result) {
  status.dataset.compileMs = result.ms.toFixed(1);
  const ms = `${Math.round(result.ms)} ms`;
  switch (result.kind) {
    case "compiled":
      if (result.warnings !== "") show("diag", result.warnings);
      startProgram(result);
      setStatus("running", `Compiled in ${ms}; running.`);
      break;
    case "failed":
      show("diag", result.text);
      show("diag", `could not compile due to ${result.errors} previous error${result.errors === 1 ? "" : "s"}\n`);
      setStatus("failed", `Failed to compile in ${ms}.`);
      break;
    case "too-long":
      tooLong();
      setStatus("too-long", `The compiler ran out of stack after ${ms}.`);
      break;
    default:
      note(MESSAGES.internal);
      show("err", `${result.text}\n`);
      setStatus("internal", `The compiler failed after ${ms}.`);
  }
}

// A fresh frame for the program: an opaque origin (`allow-scripts` and no
// `allow-same-origin`), its own policy, `<div id="app">` and the runner.
function startProgram({ js, wasm, page }) {
  const nonce = Array.from(crypto.getRandomValues(new Uint8Array(16)), (b) => b.toString(16).padStart(2, "0")).join("");
  const policy = `default-src 'none'; script-src 'nonce-${nonce}' blob: 'wasm-unsafe-eval'; worker-src blob:; connect-src 'none'`;
  frame = document.createElement("iframe");
  frame.setAttribute("sandbox", "allow-scripts");
  frame.title = "The program";
  frame.srcdoc =
    `<!doctype html><html lang="en"><head><meta charset="utf-8">` +
    `<meta http-equiv="Content-Security-Policy" content="${policy}">` +
    `<meta name="color-scheme" content="light dark"><title>The program</title></head>` +
    `<body><div id="app"></div><script nonce="${nonce}">${runner}</script></body></html>`;
  program = { js, wasm, page };
  drawing = page;
  frameBox.hidden = !page;
  frameBox.replaceChildren(frame);
  runTimer = setTimeout(() => stopProgram(MESSAGES.runLimit), RUN_LIMIT_MS);
}

// Remove the program's frame, and its worker with it, saying `why` if
// given.
function stopProgram(why) {
  clearTimeout(runTimer);
  runTimer = null;
  program = null;
  if (frame !== null) {
    frame.remove();
    frame = null;
  }
  frameBox.hidden = true;
  if (why !== undefined) {
    note(why);
    setStatus("stopped", why);
  }
}

// What the program's frame says: only its own window's messages count,
// and only as text.
addEventListener("message", (event) => {
  if (frame === null || event.source !== frame.contentWindow) return;
  const m = event.data;
  if (typeof m !== "object" || m === null) return;
  if (m.kind === "ready" && program !== null) {
    const { js, wasm, page } = program;
    program = null;
    frame.contentWindow.postMessage({ js, wasm, page }, "*", [wasm.buffer]);
  } else if (m.kind === "write") {
    show(m.fd === 2 ? "err" : "out", String(m.text));
  } else if (m.kind === "end" || m.kind === "error") {
    if (m.kind === "error") show("err", `${String(m.text)}\n`);
    if (runTimer === null) return;
    clearTimeout(runTimer);
    runTimer = null;
    // A program that draws keeps its frame, and its listeners, until Stop
    // or the next Run.
    if (!drawing) {
      frame.remove();
      frame = null;
    }
    if (m.kind === "error") setStatus("error", "The program could not run.");
    else if (drawing) setStatus("waiting", "Running in its frame; Stop removes it.");
    else setStatus("done", "Finished.");
  }
});

editor.addEventListener("keydown", (event) => {
  if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
    event.preventDefault();
    run();
  } else if (event.key === "Tab" && !event.shiftKey && !event.altKey && !event.ctrlKey && !event.metaKey) {
    event.preventDefault();
    editor.setRangeText("    ", editor.selectionStart, editor.selectionEnd, "end");
  }
});

runButton.addEventListener("click", run);

stopButton.addEventListener("click", () => {
  if (compiling !== null) {
    replaceWorker();
    note(MESSAGES.stopped);
    setStatus("stopped", MESSAGES.stopped);
  } else if (frame !== null) {
    stopProgram(MESSAGES.stopped);
  }
});

async function start() {
  startWorker();
  const [list, text] = await Promise.all([
    fetch("examples.json").then((r) => r.json()),
    fetch("runner.js").then((r) => r.text()),
  ]);
  runner = text;
  for (const example of list) examples.append(new Option(example.label, example.name));
  examples.addEventListener("change", () => {
    editor.value = list.find((e) => e.name === examples.value).source;
  });
  examples.value = "hello";
  editor.value = list.find((e) => e.name === "hello").source;
  setStatus(status.dataset.state, status.textContent);
}

start().catch((error) => setStatus("error", `The page could not start: ${error}`));
