// The gate of 4.5 in Chromium (spec
// docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md
// §5.4), run by CI's `browser` job:
//
//     node tests/browser/dev.mjs <nova> <work dir> <dir holding node_modules/playwright>
//
// It copies `examples/06-counter-spa` into the work directory, emptied
// first, and starts `nova dev --port 0` there, with `NOVA_HOME` beside it,
// reading the URL it prints. Then, through Playwright:
// 1. it clicks `+` and `-` and reads the count;
// 2. it edits `src/main.nova` from `Count:` to `Clicks:`, and waits for the
//    page to reload showing `Clicks: 0`;
// 3. it holds the page's event stream back across one more edit, from
//    `Clicks:` to `Taps:`, and waits for the page, served before that
//    build, to hear a `hello` that is not its id and reload showing
//    `Taps: 0`;
// 4. it writes a type error, and waits for `#nova-dev-overlay` with the
//    checker's message; then it reloads the page, which carries the failed
//    build's id, and checks the overlay shows again and the page does not
//    reload once more;
// 5. it restores the file, and waits for the overlay to go and the page to
//    count again.
// At each step it checks the build id in the page's reload script:
// `<start>.<number>`, the number one more for each build.
// `nova dev` is killed at the end, pass or fail. Every wait polls, for 30
// seconds at most. Playwright is the job's own install, as for `counter.mjs`.

import { spawn } from "node:child_process";
import { cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
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
      // The page is reloading.
    }
    if (Date.now() > deadline) throw new Error(`timed out waiting for ${what}`);
    await new Promise((later) => setTimeout(later, 100));
  }
}

const dir = join(work, "counter-spa");
await rm(work, { recursive: true, force: true });
await mkdir(work, { recursive: true });
await cp(join(repo, "examples", "06-counter-spa"), dir, { recursive: true });
const main = join(dir, "src", "main.nova");
const source = await readFile(main, "utf8");
const checkerSays = "text must have type `String`, found `Int`";

const dev = await startDev(dir, join(work, "nova-home"));
let browser = null;
try {
  browser = await chromium.launch();
  const page = await browser.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  const reads = (text) =>
    until(
      page,
      (t) => document.querySelector("#app .counter")?.firstChild?.textContent === t,
      text,
      `the count to read ${text}`,
    );
  const click = (label) => page.locator("#app button", { hasText: label }).click();
  // The build id the server gave the page's reload script.
  const servedId = () =>
    page.evaluate(() => document.querySelector('script[src="/__nova/reload.js"]')?.dataset.build ?? null);
  let start = null;
  const expectBuild = async (number) => {
    const id = await servedId();
    const halves = /^(\d+)\.(\d+)$/.exec(id ?? "");
    if (!halves || (start !== null && halves[1] !== start) || halves[2] !== String(number)) {
      throw new Error(`the page's build id is ${id}, not build ${number} of ${start ?? "this server"}`);
    }
    start = halves[1];
  };
  // Waits until the server serves its page with build `number`'s id.
  const served = async (number) => {
    const deadline = Date.now() + LIMIT;
    for (;;) {
      const page = await (await fetch(dev.url)).text();
      if (/data-build="\d+\.(\d+)"/.exec(page)?.[1] === String(number)) return;
      if (Date.now() > deadline) throw new Error(`timed out waiting for build ${number}`);
      await new Promise((later) => setTimeout(later, 100));
    }
  };
  const overlaid = (says) => {
    const overlay = document.getElementById("nova-dev-overlay");
    return (
      overlay !== null &&
      overlay.textContent.startsWith("nova dev: the build failed") &&
      overlay.querySelector("pre")?.textContent.includes(says) === true
    );
  };

  // 1. The counter counts.
  await page.goto(dev.url);
  await reads("Count: 0");
  await expectBuild(1);
  for (const label of ["+", "+", "+", "-"]) await click(label);
  await reads("Count: 2");

  // 2. An edit reloads the page, which starts again at 0 without the
  // marker this page had.
  await page.evaluate(() => {
    window.novaDevMarker = true;
  });
  await writeFile(main, source.replace("Count:", "Clicks:"));
  await reads("Clicks: 0");
  if (await page.evaluate(() => window.novaDevMarker === true)) {
    throw new Error("the page shows `Clicks: 0` without a reload");
  }
  await expectBuild(2);

  // 3. A page served at build 2 whose event stream connects only after
  // build 3 hears a `hello` that is not its id, and reloads.
  let release = null;
  const held = new Promise((go) => {
    release = go;
  });
  let holding = true;
  await page.route("**/__nova/events", async (route) => {
    if (holding) {
      holding = false;
      await held;
    }
    await route.continue();
  });
  await page.reload();
  await reads("Clicks: 0");
  await expectBuild(2);
  await page.evaluate(() => {
    window.novaDevMarker = true;
  });
  await writeFile(main, source.replace("Count:", "Taps:"));
  await served(3);
  release();
  await reads("Taps: 0");
  if (await page.evaluate(() => window.novaDevMarker === true)) {
    throw new Error("the page shows `Taps: 0` without a reload");
  }
  await expectBuild(3);
  await page.unroute("**/__nova/events");

  // 4. A type error shows the overlay, with the checker's message as text.
  await writeFile(main, source.replace('text("Count: ${count.value()}")', "text(1)"));
  await until(page, overlaid, checkerSays, "the overlay with the checker's message");
  // A reload now: the page carries the failed build's id, so its `hello`
  // matches and it stays, with the overlay from the `error` after it.
  await page.reload();
  await until(page, overlaid, checkerSays, "the overlay after a reload");
  await expectBuild(4);
  await page.evaluate(() => {
    window.novaDevMarker = true;
  });
  await new Promise((later) => setTimeout(later, 1500));
  if (!(await page.evaluate(() => window.novaDevMarker === true))) {
    throw new Error("the page served after the failed build reloaded again");
  }

  // 5. The fix takes the overlay away, and the page counts again.
  await writeFile(main, source);
  await until(page, () => document.getElementById("nova-dev-overlay") === null, null, "the overlay to go");
  await reads("Count: 0");
  await expectBuild(5);
  await click("+");
  await reads("Count: 1");
  if (errors.length > 0) throw new Error(`the page threw: ${errors.join("; ")}`);
  console.log(
    `dev: counts at build ${start}.1, reloads into .2 on an edit, into .3 after a missed one, shows the overlay for the type error of .4 and keeps it over a reload, and counts again at .5`,
  );
} catch (e) {
  console.error(`nova dev's stderr:\n${dev.stderr()}`);
  throw e;
} finally {
  await browser?.close();
  dev.kill();
}
