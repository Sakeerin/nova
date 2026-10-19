// The router's browser test (spec
// docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md
// §5.4, the user's decision 8), run by CI's `browser` job:
//
//     node tests/browser/router.mjs <nova> <work dir> <dir holding node_modules/playwright>
//
// It copies `tests/browser/router-app`, a `HistoryRouter` over `/`,
// `/posts/:id`, `/search` and `*`, into the work directory, emptied first,
// and starts `nova dev --port 0` there, with `NOVA_HOME` beside it, reading
// the URL it prints. The app has no `index.html` of its own, so the page is
// the one the compiler generates, titled `Nova`, which loads `/app.js` by
// its absolute path (the user's decision 9 of 2026-10-18): step 3's reload
// at `/posts/2` proves it. It sets a marker on `window` that a full reload
// would lose. Then, through Playwright:
// 1. it clicks a link: the URL is `/posts/2`, the post shows, and the
//    marker is still there;
// 2. it goes back: the home page shows, the marker still there;
// 3. it goes forward and reloads at `/posts/2`: the fallback serves the
//    page, and the post shows, the marker gone; the page carries the same
//    build id, `<start>.1`, in its reload script, so it stays;
// 4. it opens `/nowhere`: the "not found" route shows;
// 5. it follows a link with a query: `use_query` reads it.
// `nova dev` is killed at the end, pass or fail. Every wait polls, for 30
// seconds at most. Playwright is the job's own install, as for `counter.mjs`.

import { spawn } from "node:child_process";
import { cp, mkdir, rm } from "node:fs/promises";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { createInterface } from "node:readline";
import { fileURLToPath } from "node:url";

const [nova, work, modules] = process.argv.slice(2).map((p) => resolve(p));
const { chromium } = createRequire(join(modules, "noop.js"))("playwright");
const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const LIMIT = 30000;

// `nova dev --port 0` in `dir`: its URL, its stderr so far, and a kill.
async function startDev(dir, home) {
  const child = spawn(nova, ["dev", "--port", "0"], {
    cwd: dir,
    env: { ...process.env, NOVA_HOME: home, NO_COLOR: "1" },
    stdio: ["ignore", "pipe", "pipe"],
  });
  let stderr = "";
  child.stderr.on("data", (chunk) => {
    stderr += chunk;
  });
  const dev = { url: null, stderr: () => stderr, kill: () => child.kill() };
  try {
    dev.url = await new Promise((found, failed) => {
      const timer = setTimeout(() => failed(new Error("no URL within 30 s")), LIMIT);
      createInterface({ input: child.stdout }).on("line", (line) => {
        const url = /^nova dev: serving (http:\/\/127\.0\.0\.1:\d+\/)$/.exec(line)?.[1];
        if (url) {
          clearTimeout(timer);
          found(url);
        }
      });
      child.on("exit", (code) => failed(new Error(`nova dev exited with ${code}`)));
    });
  } catch (e) {
    child.kill();
    throw new Error(`${e.message}; nova dev's stderr:\n${stderr}`);
  }
  return dev;
}

// Polls `test(arg)` in the page until it holds; a reload between two polls
// is only a poll that fails.
async function until(page, test, arg, what) {
  const deadline = Date.now() + LIMIT;
  for (;;) {
    try {
      if (await page.evaluate(test, arg)) return;
    } catch {
      // The page is loading.
    }
    if (Date.now() > deadline) {
      const now = await page.evaluate(() => document.getElementById("title")?.textContent).catch(() => "?");
      throw new Error(`timed out waiting for ${what}; the title reads ${JSON.stringify(now)} at ${page.url()}`);
    }
    await new Promise((later) => setTimeout(later, 100));
  }
}

const dir = join(work, "router-app");
await rm(work, { recursive: true, force: true });
await mkdir(work, { recursive: true });
await cp(join(repo, "tests", "browser", "router-app"), dir, { recursive: true });

const dev = await startDev(dir, join(work, "nova-home"));
let browser = null;
try {
  browser = await chromium.launch();
  const page = await browser.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  const shows = (title) =>
    until(page, (t) => document.getElementById("title")?.textContent === t, title, `the title ${title}`);
  const marked = () => page.evaluate(() => window.novaRouterMarker === true);
  const path = () => new URL(page.url()).pathname + new URL(page.url()).search;
  const servedId = () =>
    page.evaluate(() => document.querySelector('script[src="/__nova/reload.js"]')?.dataset.build ?? null);
  const expectPath = (expected) => {
    if (path() !== expected) throw new Error(`the URL is ${page.url()}, not ${expected}`);
  };

  await page.goto(dev.url);
  await shows("Home");
  const title = await page.title();
  if (title !== "Nova") throw new Error(`the page is titled ${JSON.stringify(title)}, not the generated page's "Nova"`);
  const id = await servedId();
  if (!/^\d+\.1$/.test(id ?? "")) throw new Error(`the page's build id is ${id}, not the first build's`);
  await page.evaluate(() => {
    window.novaRouterMarker = true;
  });

  // 1. A link's click navigates without a reload.
  await page.click("#post");
  await shows("Post 2");
  expectPath("/posts/2");
  if (!(await marked())) throw new Error("the link's click reloaded the page");

  // 2. The back button goes home, still without a reload.
  await page.goBack();
  await shows("Home");
  expectPath("/");
  if (!(await marked())) throw new Error("the back button reloaded the page");

  // 3. A reload at `/posts/2`: the fallback serves the page, and the
  // router shows the post.
  await page.goForward();
  await shows("Post 2");
  expectPath("/posts/2");
  await page.reload();
  await shows("Post 2");
  expectPath("/posts/2");
  if (await marked()) throw new Error("the reload kept the marker: the page did not reload");
  if ((await servedId()) !== id) throw new Error(`the reloaded page's build id is ${await servedId()}, not ${id}`);
  await page.evaluate(() => {
    window.novaRouterMarker = true;
  });
  await new Promise((later) => setTimeout(later, 1000));
  if (!(await marked())) throw new Error("the page reloaded again after its hello");

  // 4. A path no route matches but `*`.
  await page.goto(new URL("/nowhere", dev.url).href);
  await shows("Not found");
  expectPath("/nowhere");

  // 5. A link with a query: `use_query` reads it, `+` as a space.
  await page.click("#search");
  await shows("Search for nova router, page 3");
  expectPath("/search?q=nova+router&page=3");

  if (errors.length > 0) throw new Error(`the page threw: ${errors.join("; ")}`);
  console.log(
    `router: a link to /posts/2 without a reload, back home, a reload at /posts/2 that stays at build ${id}, /nowhere not found, and a query read`,
  );
} catch (e) {
  console.error(`nova dev's stderr:\n${dev.stderr()}`);
  throw e;
} finally {
  await browser?.close();
  dev.kill();
}
