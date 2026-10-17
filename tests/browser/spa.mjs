// The gate of 4.4 in Chromium (spec
// docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md
// §9.4), run by CI's `browser` job on `examples/06-counter-spa`, built with
// its own page:
//
//     node tests/browser/spa.mjs <dist> <dir holding node_modules/playwright>
//
// It serves `dist/` from 127.0.0.1, with the module as `application/wasm`,
// opens the page, checks the count reads 0, clicks `+` three times and `-`
// once, finding the buttons by their text, and checks the count reads 2 and
// the console holds no error. Playwright is the job's own install, as for `counter.mjs`.

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
  const button = (label) => page.locator("#app button", { hasText: label });
  const reads = (text) =>
    page.waitForFunction(
      (t) => document.querySelector("#app .counter")?.firstChild?.textContent === t,
      text,
      { timeout: 10000 },
    );
  await reads("Count: 0");
  for (const label of ["+", "+", "+", "-"]) await button(label).click();
  await reads("Count: 2");
  if (errors.length > 0) throw new Error(`the console has errors: ${errors.join("; ")}`);
  console.log("counter-spa: +, +, +, -, and the count reads 2");
} finally {
  await browser.close();
  server.close();
}
