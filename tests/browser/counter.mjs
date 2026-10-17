// The gate's counter in Chromium (spec
// docs/superpowers/specs/2026-10-16-phase-4-2-std-dom-and-the-shim-design.md
// §10.3), run by CI's `browser` job on 4.2's `tests/dom/counter.nova` and on
// 4.3's builder counter, `tests/dom/ui_counter.nova`, which has the same ids
// (spec docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md
// §9.4):
//
//     node tests/browser/counter.mjs <dist> <dir holding node_modules/playwright>
//
// It serves `dist/` from 127.0.0.1, with the module as `application/wasm`,
// opens the page, clicks the button three times, and checks the count reads
// 3 and the console holds no error. Playwright is the job's own install:
// nothing in `cargo test` or `nova` installs a package.

import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { extname, join, resolve, sep } from "node:path";

const [dist, modules] = process.argv.slice(2).map((p) => resolve(p));
const { chromium } = createRequire(join(modules, "noop.js"))("playwright");

const types = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".json": "application/json",
  ".wasm": "application/wasm",
};

// `dist/`'s files, and nothing outside it.
const server = createServer(async (req, res) => {
  const path = decodeURIComponent(new URL(req.url, "http://127.0.0.1").pathname);
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
const url = `http://127.0.0.1:${server.address().port}/`;

const browser = await chromium.launch();
const errors = [];
try {
  const page = await browser.newPage();
  page.on("console", (m) => {
    if (m.type() === "error") errors.push(m.text());
  });
  page.on("pageerror", (e) => errors.push(String(e)));
  await page.goto(url);
  await page.locator("#add").waitFor();
  for (let i = 0; i < 3; i++) await page.click("#add");
  await page.waitForFunction(() => document.getElementById("count").textContent === "3", null, {
    timeout: 10000,
  });
  if (errors.length > 0) throw new Error(`the console has errors: ${errors.join("; ")}`);
  console.log("counter: three clicks, and the count reads 3");
} finally {
  await browser.close();
  server.close();
}
