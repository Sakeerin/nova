// The gate of 4.6 in Chromium (spec
// docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
// §9.5, the user's decision 11), run by CI's `browser` job:
//
//     node tests/browser/ssr.mjs <nova> <work dir> <dir holding node_modules/playwright>
//
// It copies `tests/browser/ssr-app`, a project with a client and a server,
// into the work directory, emptied first, and builds it there with `nova
// build --ssg`, with `NOVA_HOME` beside it: `dist/` holds the client, the
// generated page titled `Nova`, and the pages SSG wrote for `/` and
// `/about`, which `manifest.json` lists, and none for `/posts/:id`. It starts
// the server, `target/debug/ssr-app-server`, which binds `127.0.0.1:0`, and
// reads the port from the line `ssr: serving http://127.0.0.1:<port>/`.
//
// A page "hydrates in place" when, opened with `app.wasm` held back
// (`page.route`), its `#root` reads `data-state="server"`, and every node
// under `#app`, elements, texts and comments, is kept as a handle; then,
// once `app.wasm` loads and the client's `on_mount` sets `data-state` to
// `hydrated`, which it does after hydration's walk, each handle is `===` the
// node at its place in the page, with no node more or less. Through
// Playwright:
// 1. `/` answers `200` and hydrates in place; a click on `+` updates the
//    count's own text node, kept from the server; the `show` hides and
//    shows again;
// 2. a link goes to `/about`, and a marker on `window` that a reload would
//    lose is still there;
// 3. `/posts/2` answers `200` and hydrates in place, showing the post;
// 4. `/nowhere` answers `404` with the "not found" page, which hydrates in
//    place too;
// 5. a static Node server, as `counter.mjs`'s, serves `dist/`; it answers
//    `/about/` with `dist/about/index.html`, which already holds the
//    rendered page and its data, and that page hydrates in place.
// On every page the title the server's data carries, which holds
// `</script>`, `<!--` and `&`, reads as written; the console has no error
// but the one Chromium logs for `/nowhere`'s own `404`, and the page throws
// none. The summary line ends with the client's size, `app.wasm` and
// `app.js` gzipped, printed and not limited (spec §12). The server is
// killed at the end, pass or fail. Every wait polls, for 30 seconds at
// most. Playwright is the job's own install, as for `counter.mjs`.

import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { cp, mkdir, readFile, rm, stat } from "node:fs/promises";
import { createRequire } from "node:module";
import { dirname, extname, join, resolve, sep } from "node:path";
import { createInterface } from "node:readline";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";

const [nova, work, modules] = process.argv.slice(2).map((p) => resolve(p));
const { chromium } = createRequire(join(modules, "noop.js"))("playwright");
const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const LIMIT = 30000;
const TITLE = "Guests </script><!-- & friends";

// Runs `command` in `dir` to its end: its status and its output.
function run(command, args, dir, env) {
  return new Promise((done, failed) => {
    const child = spawn(command, args, { cwd: dir, env, stdio: ["ignore", "pipe", "pipe"] });
    let output = "";
    child.stdout.on("data", (chunk) => {
      output += chunk;
    });
    child.stderr.on("data", (chunk) => {
      output += chunk;
    });
    child.on("error", failed);
    child.on("exit", (code) => done({ code, output }));
  });
}

// The server in `dir`: its URL, its stderr so far, and a kill.
async function startServer(dir, env) {
  const exe = process.platform === "win32" ? ".exe" : "";
  const child = spawn(join(dir, "target", "debug", `ssr-app-server${exe}`), [], {
    cwd: dir,
    env,
    stdio: ["ignore", "pipe", "pipe"],
  });
  let stderr = "";
  child.stderr.on("data", (chunk) => {
    stderr += chunk;
  });
  const server = { url: null, stderr: () => stderr, kill: () => child.kill() };
  try {
    server.url = await new Promise((found, failed) => {
      const timer = setTimeout(() => failed(new Error("no URL within 30 s")), LIMIT);
      createInterface({ input: child.stdout }).on("line", (line) => {
        const url = /^ssr: serving (http:\/\/127\.0\.0\.1:\d+\/)$/.exec(line)?.[1];
        if (url) {
          clearTimeout(timer);
          found(url);
        }
      });
      child.on("exit", (code) => failed(new Error(`the server exited with ${code}`)));
    });
  } catch (e) {
    child.kill();
    throw new Error(`${e.message}; the server's stderr:\n${stderr}`);
  }
  return server;
}

// A static server of `dist/`'s files, and nothing outside it, as
// `counter.mjs`'s: a path ending in `/` names its directory's `index.html`.
const types = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".json": "application/json",
  ".wasm": "application/wasm",
};
async function startStatic(dist) {
  const server = createServer(async (req, res) => {
    const path = decodeURIComponent(new URL(req.url, "http://127.0.0.1").pathname);
    const file = resolve(join(dist, path.endsWith("/") ? `${path}index.html` : path));
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
  return { url: `http://127.0.0.1:${server.address().port}/`, close: () => server.close() };
}

// Polls `test(arg)` in the page until it holds.
async function until(page, test, arg, what) {
  const deadline = Date.now() + LIMIT;
  for (;;) {
    try {
      if (await page.evaluate(test, arg)) return;
    } catch {
      // The page is loading.
    }
    if (Date.now() > deadline) {
      const now = await page.evaluate(() => document.getElementById("page")?.textContent).catch(() => "?");
      throw new Error(
        `timed out waiting for ${what}; the page's heading reads ${JSON.stringify(now)} at ${page.url()}`,
      );
    }
    await new Promise((later) => setTimeout(later, 100));
  }
}

const heading = (page) => page.evaluate(() => document.getElementById("page")?.textContent ?? null);
const state = (page) => page.evaluate(() => document.getElementById("root")?.dataset.state ?? null);

// Opens `url` with `app.wasm` held back, checks the server's HTML is there,
// keeps a handle to each node under `#app`, lets `app.wasm` load, waits for
// hydration, and checks every handle is still the page's node, in place.
// Returns the response's status, the handles, and how many there are.
async function hydratesInPlace(page, url, expected) {
  let release;
  const held = new Promise((go) => {
    release = go;
  });
  await page.route("**/app.wasm", async (route) => {
    await held;
    await route.continue();
  });
  try {
    const response = await page.goto(url, { waitUntil: "domcontentloaded" });
    if ((await state(page)) !== "server") {
      throw new Error(`${url}: before app.wasm loads, #root's data-state is ${await state(page)}, not "server"`);
    }
    if ((await heading(page)) !== expected) {
      throw new Error(`${url}: the server's HTML shows ${JSON.stringify(await heading(page))}, not ${expected}`);
    }
    const kept = await page.evaluateHandle(() => window.novaNodes());
    const count = await kept.evaluate((nodes) => nodes.length);
    release();
    await until(page, () => document.getElementById("root")?.dataset.state === "hydrated", null, `${url} to hydrate`);
    const moved = await page.evaluate((nodes) => {
      const now = window.novaNodes();
      if (now.length !== nodes.length) return `${nodes.length} nodes became ${now.length}`;
      const at = nodes.findIndex((node, i) => node !== now[i]);
      return at < 0 ? null : `node ${at}, ${nodes[at].nodeName}, is not the server's`;
    }, kept);
    if (moved) throw new Error(`${url} did not hydrate in place: ${moved}`);
    if ((await heading(page)) !== expected) {
      throw new Error(`${url}: the hydrated page shows ${JSON.stringify(await heading(page))}, not ${expected}`);
    }
    const title = await page.evaluate(() => document.getElementById("title")?.textContent);
    if (title !== TITLE) throw new Error(`${url}: the data's title reads ${JSON.stringify(title)}`);
    if ((await page.title()) !== "Nova") {
      throw new Error(
        `${url}: the page is titled ${JSON.stringify(await page.title())}, not the generated page's "Nova"`,
      );
    }
    return { status: response.status(), kept, count };
  } finally {
    release();
    await page.unroute("**/app.wasm");
  }
}

const dir = join(work, "ssr-app");
const dist = join(dir, "dist");
const env = { ...process.env, NOVA_HOME: join(work, "nova-home"), NO_COLOR: "1" };
await rm(work, { recursive: true, force: true });
await mkdir(work, { recursive: true });
await cp(join(repo, "tests", "browser", "ssr-app"), dir, { recursive: true });

const build = await run(nova, ["build", "--ssg"], dir, env);
if (build.code !== 0) throw new Error(`nova build --ssg exited with ${build.code}:\n${build.output}`);
const listed = Object.keys(JSON.parse(await readFile(join(dist, "manifest.json"), "utf8")).files);
if (!listed.includes("about/index.html")) throw new Error(`the manifest lists ${listed.join(", ")}`);
if (await stat(join(dist, "posts")).catch(() => null)) throw new Error("SSG wrote a page for /posts/:id");

const server = await startServer(dir, env);
let browser = null;
let files = null;
try {
  browser = await chromium.launch();
  const page = await browser.newPage();
  const errors = [];
  // The one error a `404` page logs, the browser's own for its document.
  let missing = null;
  page.on("console", (m) => {
    if (m.type() !== "error") return;
    if (m.location().url === missing && /status of 404/.test(m.text())) return;
    errors.push(m.text());
  });
  page.on("pageerror", (e) => errors.push(String(e)));
  const clean = (what) => {
    if (errors.length > 0) throw new Error(`${what}: the console has errors: ${errors.join("; ")}`);
  };
  await page.addInitScript(() => {
    window.novaNodes = () => {
      const walker = document.createTreeWalker(document.getElementById("app"), NodeFilter.SHOW_ALL);
      const nodes = [];
      while (walker.nextNode()) nodes.push(walker.currentNode);
      return nodes;
    };
  });
  const path = () => new URL(page.url()).pathname;

  // 1. `/`: in place, then a click counts in the server's own text node, and
  // the `show` builds again between the server's markers.
  const home = await hydratesInPlace(page, server.url, "Home");
  if (home.status !== 200) throw new Error(`/ answered ${home.status}`);
  await page.click("#add");
  await until(page, () => document.getElementById("count")?.textContent === "Clicked 1 times", null, "the count");
  const counted = await page.evaluate((nodes) => {
    const p = document.getElementById("count");
    return nodes.includes(p) && nodes.includes(p.firstChild) && p.firstChild.data === "Clicked 1 times";
  }, home.kept);
  if (!counted) throw new Error("the click did not update the server's nodes");
  await page.click("#toggle");
  await until(page, () => document.getElementById("details") === null, null, "the show to hide");
  await page.click("#toggle");
  await until(page, () => document.getElementById("details")?.textContent === "Details", null, "the show to show");
  clean("/");

  // 2. A link's click goes to `/about` without a reload.
  await page.evaluate(() => {
    window.novaSsrMarker = true;
  });
  await page.click("#about");
  await until(page, () => document.getElementById("page")?.textContent === "About", null, "the about page");
  if (path() !== "/about") throw new Error(`the URL is ${page.url()}, not /about`);
  const marked = await page.evaluate(() => window.novaSsrMarker === true);
  if (!marked) throw new Error("the link's click reloaded the page");
  clean("the link to /about");

  // 3. A route with a parameter, rendered by the server.
  const post = await hydratesInPlace(page, new URL("/posts/2", server.url).href, "Post 2");
  if (post.status !== 200) throw new Error(`/posts/2 answered ${post.status}`);
  clean("/posts/2");

  // 4. A path only the catch-all matches.
  missing = new URL("/nowhere", server.url).href;
  const nowhere = await hydratesInPlace(page, missing, "Not found");
  if (nowhere.status !== 404) throw new Error(`/nowhere answered ${nowhere.status}, not 404`);
  clean("/nowhere");

  // 5. SSG's page for `/about`, from a static server.
  files = await startStatic(dist);
  const about = new URL("/about/", files.url).href;
  const fetched = await (await fetch(about)).text();
  if (fetched !== (await readFile(join(dist, "about", "index.html"), "utf8"))) {
    throw new Error("the static server's /about/ is not dist/about/index.html");
  }
  if (!fetched.includes('<h2 id="page">About</h2>') || !fetched.includes('id="nova-data"')) {
    throw new Error(`dist/about/index.html was not rendered:\n${fetched}`);
  }
  const generated = await hydratesInPlace(page, about, "About");
  clean("SSG's /about/");

  // The client's size, printed and not limited (spec §12, risk 4), measured
  // as the size test measures 06's: each file gzipped at the best level.
  let gzipped = 0;
  for (const name of ["app.wasm", "app.js"]) {
    gzipped += gzipSync(await readFile(join(dist, name)), { level: 9 }).length;
  }

  console.log(
    `ssr: / hydrates in place (${home.count} nodes kept) and counts, a link to /about without a reload, /posts/2 in place (${post.count}), /nowhere 404 in place (${nowhere.count}), and SSG's /about/ arrives rendered and hydrates in place (${generated.count}); the client is ${gzipped} bytes gzipped`,
  );
} catch (e) {
  console.error(`the server's stderr:\n${server.stderr()}`);
  throw e;
} finally {
  await browser?.close();
  files?.close();
  server.kill();
}
