// The playground's page in Chromium (spec
// docs/superpowers/specs/2026-11-28-phase-6-1-the-playground-design.md
// §5, test 7), run by CI's Playground job on the page `build.mjs` built:
//
//     node tests/browser/playground.mjs <page> <dir holding node_modules/playwright>
//
// It serves `<page>` from 127.0.0.1, port 0, as `counter.mjs` serves its
// `dist/`, and drives the page as a reader would. It prints when the
// compiler was ready and hello world's first compile, read from the status
// line's data attributes, and fails if that compile took 2,000 ms or more
// (B12). Then: each console example's output; type_error's diagnostics and
// closing line; the counter clicked to `Count: 2` in its frame, whose
// origin is opaque; the depth cases of `tests/playground/depth.mjs`
// through the editor, compiled in the page's worker, where the stack is
// thinnest, failing on any throw at 128; 10,000 `if`s showing the too-long
// message, then hello world; a loop stopped by Stop, then by the 10 s
// limit, its worker gone each time and the page answering throughout; a
// `std/dom` `fetch` to this server failing, the server seeing no request;
// and every request one of the page's files, none to another host.
// Playwright is the job's own install, as for `counter.mjs`.

import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { extname, join, resolve, sep } from "node:path";
import { parseKinds, typedKinds } from "../playground/depth.mjs";

const started = Date.now();
if (process.argv.length !== 4) {
  throw new Error("usage: node tests/browser/playground.mjs <page> <dir holding node_modules/playwright>");
}
const [dist, modules] = process.argv.slice(2).map((p) => resolve(p));
const { chromium } = createRequire(join(modules, "noop.js"))("playwright");

const types = {
  ".html": "text/html",
  ".css": "text/css",
  ".js": "text/javascript",
  ".json": "application/json",
  ".wasm": "application/wasm",
};
const FILES = ["/", "/index.html", "/style.css", "/page.js", "/compiler.js", "/worker.js", "/runner.js", "/nova_playground.wasm", "/examples.json"];

// `<page>`'s files, and nothing outside it; every request is kept.
const requests = [];
const server = createServer(async (req, res) => {
  const path = decodeURIComponent(new URL(req.url, "http://127.0.0.1").pathname);
  requests.push(path);
  const file = resolve(join(dist, path === "/" ? "index.html" : path));
  if (file !== dist && !file.startsWith(dist + sep)) {
    res.writeHead(403).end();
    return;
  }
  try {
    const body = await readFile(file);
    res.writeHead(200, { "content-type": types[extname(file)] ?? "application/octet-stream" });
    res.end(body);
  } catch {
    res.writeHead(404).end();
  }
});
await new Promise((ready) => server.listen(0, "127.0.0.1", ready));
const origin = `http://127.0.0.1:${server.address().port}`;
const examples = JSON.parse(await readFile(join(dist, "examples.json"), "utf8"));
const example = (name) => examples.find((e) => e.name === name);

function check(ok, what) {
  if (!ok) throw new Error(what);
}

const TOO_LONG =
  "The playground's compiler ran out of stack on this program: one of its functions is probably too long for the compiler in the browser. Splitting it into smaller functions should help, and nova build --target wasm compiles it as it is on your own machine.\n";
const STOPPED = "Stopped.\n";
const RUN_LIMIT = "Stopped: the program was still running after 10 seconds.\n";
// The program fetches this path from the test's own server.
const PROBE = "/from-the-program";

const browser = await chromium.launch();
const errors = [];
const sent = [];
const workers = new Set();
try {
  const page = await browser.newPage();
  page.on("console", (m) => {
    // The program's refused `fetch` is the one error expected.
    if (m.type() === "error" && !(m.text().includes("Content Security Policy") && m.text().includes(PROBE))) errors.push(m.text());
  });
  page.on("pageerror", (e) => errors.push(String(e)));
  page.on("request", (r) => sent.push(r.url()));
  // The program's workers, started in its frame from `blob:` URLs.
  page.on("worker", (w) => {
    if (!w.url().startsWith("blob:")) return;
    workers.add(w);
    w.on("close", () => workers.delete(w));
  });

  const status = page.locator("#status");
  const state = () => status.getAttribute("data-state");
  await page.goto(`${origin}/`);
  await page.waitForFunction(() => !document.getElementById("run").disabled, null, { timeout: 30000 });
  const readyMs = Number(await status.getAttribute("data-ready-ms"));

  // What the output pane shows, by kind.
  const pane = () =>
    page.evaluate(() => {
      const pick = (kind) => Array.from(document.querySelectorAll(`#output span.${kind}`), (s) => s.textContent).join("");
      return { out: pick("out"), err: pick("err"), diag: pick("diag"), note: pick("note") };
    });
  // Run what the editor holds, by `how` (the button, or the keys), and
  // wait until the Run is past compiling and running.
  let runs = 0;
  async function run(how = "button") {
    runs += 1;
    if (how === "keys") await page.press("#editor", "Control+Enter");
    else await page.click("#run");
    await page.waitForFunction(
      (n) => {
        const s = document.getElementById("status").dataset;
        return s.run === String(n) && s.state !== "compiling" && s.state !== "running";
      },
      runs,
      { timeout: 30000 },
    );
    return { state: await state(), ...(await pane()) };
  }
  // Put `source` in the editor, as a paste would: `fill` types too slowly
  // for 10,000 lines.
  const edit = (source) =>
    page.$eval(
      "#editor",
      (editor, text) => {
        editor.value = text;
        editor.dispatchEvent(new Event("input", { bubbles: true }));
      },
      source,
    );
  async function runSource(source, how) {
    await edit(source);
    return run(how);
  }
  async function runExample(name) {
    await page.selectOption("#examples", name);
    return run();
  }

  // Hello world, which the editor opens on: its first compile.
  check((await page.inputValue("#editor")) === example("hello").source, "the editor does not open on hello world");
  const hello = await run();
  const firstMs = Number(await status.getAttribute("data-compile-ms"));
  check(hello.state === "done" && hello.out === "Hello, World!\n" && hello.err === "", `hello: ${JSON.stringify(hello)}`);
  console.log(`playground: the compiler ready ${readyMs.toFixed(0)} ms after navigation; hello world's first compile ${firstMs.toFixed(1)} ms`);
  check(firstMs < 2000, `hello world's first compile took ${firstMs} ms, not under 2,000`);

  // The editor: Tab puts four spaces.
  await page.locator("#editor").fill("");
  await page.press("#editor", "Tab");
  check((await page.inputValue("#editor")) === "    ", "Tab does not put four spaces");

  // The fixed list.
  for (const { name, expected } of examples) {
    if (name === "counter" || name === "type_error") continue;
    const r = await runExample(name);
    check(r.state === "done" && r.out === expected && r.err === "" && r.note === "", `${name}: ${JSON.stringify(r)}`);
  }
  const typeError = await runExample("type_error");
  check(
    typeError.state === "failed" && typeError.diag === `${example("type_error").expected}could not compile due to 1 previous error\n`,
    `type_error: ${JSON.stringify(typeError)}`,
  );
  const counter = await runExample("counter");
  check(counter.state === "waiting" && (await page.locator("#frame").isVisible()), `counter: ${JSON.stringify(counter)}`);
  const programFrame = page.frames().find((f) => f !== page.mainFrame());
  check((await programFrame.evaluate(() => self.origin)) === "null", "the program's frame has an origin, not an opaque one");
  const frame = page.frameLocator("#frame iframe");
  const reads = (text) =>
    frame.locator("#app .counter").evaluate((el, t) => el.firstChild?.textContent === t, text);
  await frame.locator("#app .counter").waitFor();
  check(await reads("Count: 0"), "the counter does not read Count: 0");
  for (const label of ["+", "+", "+", "-"]) await frame.locator("#app button", { hasText: label }).click();
  await frame.locator("#app .counter").filter({ hasText: "Count: 2" }).waitFor({ timeout: 10000 });

  // The depth cases, through the editor, in the worker.
  const codes = (text) => Array.from(text.matchAll(/error\[([A-Z]\d{4})\]/g), (m) => m[1]);
  let ranRight = 0;
  for (const [kind, make] of typedKinds) {
    const at128 = make(128);
    const r = await runSource(at128.source);
    if (at128.code !== undefined) {
      const found = codes(r.diag);
      check(r.state === "failed" && found.length > 0 && found.every((c) => c === at128.code), `typed ${kind} at 128: ${JSON.stringify(r)}`);
    } else {
      check(r.state === "done" && r.out === at128.prints && r.err === "", `typed ${kind} at 128: ${JSON.stringify(r)}`);
      ranRight += 1;
    }
    const at129 = await runSource(make(129).source);
    check(at129.state === "failed" && codes(at129.diag).includes("P0002"), `typed ${kind} at 129: ${JSON.stringify(at129)}`);
  }
  for (const [kind, make] of parseKinds) {
    const r = await runSource(make(128));
    check(r.state === "failed" && !codes(r.diag).includes("P0002"), `5.1b's ${kind} at 128: ${JSON.stringify(r)}`);
    const at129 = await runSource(make(129));
    check(at129.state === "failed" && codes(at129.diag).includes("P0002"), `5.1b's ${kind} at 129: ${JSON.stringify(at129)}`);
  }
  console.log(
    `playground: depth in the worker: ${2 * (typedKinds.length + parseKinds.length)} cases, no throw, ` +
      `${ranRight} kinds ran right at 128, all P0002 at 129`,
  );

  // A long function body, then hello world, by the keys.
  const ifs = `fn main() {\n    let x = 3\n    let mut y = 0\n${Array.from({ length: 10_000 }, (_, k) => `    if x > ${k % 7} { y = y + 1 }\n`).join("")}    println("\${y}")\n}\n`;
  const long = await runSource(ifs);
  check(long.state === "too-long" && long.note === TOO_LONG, `10,000 ifs: ${JSON.stringify(long)}`);
  const again = await runSource(example("hello").source, "keys");
  check(again.state === "done" && again.out === "Hello, World!\n", `hello after the overflow: ${JSON.stringify(again)}`);

  // A loop that never ends: Stop, then the limit. The page answers while
  // it runs, and the program's worker goes with its frame.
  const loop = "fn main() {\n    let mut i = 0\n    while true {\n        i = i + 1\n    }\n}\n";
  let slowest = 0;
  async function answering(ms) {
    const until = Date.now() + ms;
    while (Date.now() < until) {
      const t = Date.now();
      await page.evaluate(() => document.getElementById("status").dataset.state);
      slowest = Math.max(slowest, Date.now() - t);
      await new Promise((r) => setTimeout(r, 100));
    }
  }
  async function workersAre(n) {
    const until = Date.now() + 5000;
    while (workers.size !== n && Date.now() < until) await new Promise((r) => setTimeout(r, 50));
    return workers.size === n;
  }
  const workerGone = () =>
    page.waitForFunction(() => document.querySelector("#frame iframe") === null, null, { timeout: 5000 }).then(async () => {
      check(await workersAre(0), "the program's worker outlived its frame");
    });
  check(await workersAre(0), "a program's worker outlived its Run");
  await edit(loop);
  runs += 1;
  await page.click("#run");
  await page.waitForFunction(() => document.getElementById("status").dataset.state === "running", null, { timeout: 30000 });
  check(await workersAre(1), `the loop runs in ${workers.size} workers, not one`);
  await answering(1000);
  await page.click("#stop");
  check((await state()) === "stopped" && (await pane()).note === STOPPED, `Stop: ${JSON.stringify(await pane())}`);
  await workerGone();
  runs += 1;
  await page.click("#run");
  const limitStart = Date.now();
  await page.waitForFunction(() => document.getElementById("status").dataset.state === "running", null, { timeout: 30000 });
  await answering(9000);
  await page.waitForFunction(() => document.getElementById("status").dataset.state === "stopped", null, { timeout: 15000 });
  const limitMs = Date.now() - limitStart;
  check((await pane()).note === RUN_LIMIT, `the limit: ${JSON.stringify(await pane())}`);
  await workerGone();
  check(slowest < 1000, `the page took ${slowest} ms to answer while the loop ran`);
  console.log(`playground: the loop stopped by Stop, then by the limit after ${limitMs} ms; the page answered within ${slowest} ms`);

  // The program's own network: none.
  const fetching = `import dom\n\nasync fn main() {\n    match fetch("${origin}${PROBE}").await {\n        Ok(res) => println("fetched: \${res.status}")\n        Err(e) => println("failed: \${e.message}")\n    }\n}\n`;
  const fetched = await runSource(fetching);
  check(fetched.out.startsWith("failed: ") && fetched.err === "", `the program's fetch: ${JSON.stringify(fetched)}`);
  await new Promise((r) => setTimeout(r, 500));
  check(!requests.includes(PROBE), "the server saw the program's request");

  // Every request one of the page's files, none to another host.
  const strays = requests.filter((p) => !FILES.includes(p));
  check(strays.length === 0, `requests for files not the page's: ${strays.join(", ")}`);
  const away = sent.filter((u) => /^https?:/.test(u) && !u.startsWith(`${origin}/`));
  check(away.length === 0, `requests to another host: ${away.join(", ")}`);
  check(errors.length === 0, `the console has errors: ${errors.join("; ")}`);
  console.log(`playground: ${runs} Runs, ${requests.length} requests, all for the page's files; ${Date.now() - started} ms in all`);
} finally {
  await browser.close();
  server.close();
}
