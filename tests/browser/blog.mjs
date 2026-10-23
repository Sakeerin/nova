// The gate of 4.7 in Chromium (spec
// docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md
// §7.5), run by CI's `browser` job:
//
//     node tests/browser/blog.mjs <nova> <work dir> <dir holding node_modules/playwright>
//
// It copies `examples/07-fullstack-blog` into the work directory, emptied
// first, leaving out what a build or a run there may have left (`dist/`,
// `target/`, `data/blog.json`, `data/users.json`, each with its `.tmp`,
// and `data/secret`), and builds it with `nova build --ssr`, with
// `NOVA_HOME` beside it, as `crates/nova-cli/tests/blog_tests.rs` does.
// It sets the admin's password through
// `fullstack-blog-server --set-password admin --iterations 1000`, the
// password on standard input, and starts the server on
// `--address 127.0.0.1:0`, reading the port from the line
// `blog: serving http://127.0.0.1:<port>/`.
//
// A page "hydrates in place" when, opened with `app.wasm` held back
// (`page.route`), its `#root` reads `data-state="server"`, each form's
// submit button is there and enabled, so the form works without
// JavaScript, and every node under `#app` is kept as a handle; then, once `app.wasm` loads and the
// app's `on_mount` sets `data-state` to `hydrated`, after hydration's walk,
// each handle but those inside a form's `.actions` is still in the page, in
// the same order. The submit button inside `.actions` is the one thing the
// client may swap: a form whose fields are blank gets the disabled button
// in place of the server's enabled one. Through Playwright:
// 1. `/`, `/posts/1` and `/login` answer `200` and hydrate in place;
//    `/nowhere` and `/posts/99` answer `404` and hydrate in place;
// 2. on `/posts/1`, the comment form's button is disabled while a field is
//    blank, with the field's message under it, and enabled once both are
//    filled; the comment then posts, and the post's page shows it;
// 3. a comment with a blank text, sent by the form's `submit()` so the
//    client's checks do not stop it, is answered `400` with the post's page
//    at `/posts/1/comments`, the path it was posted to, which hydrates in
//    place with the name kept and the text's message;
// 4. a comment whose text holds a NUL posts, and comes back without it,
//    and the post's page still hydrates in place;
// 5. a comment whose name and text hold a script that would set
//    `window.novaXss` and open a dialog posts too: neither happens, and its
//    text shows as written;
// 6. the comment form, its hidden `csrf` field removed, is answered `403`
//    with the page saying the form has expired, at `/posts/1/comments`,
//    which hydrates in place, and the comment is not added;
// 7. a login with a wrong password is answered `401` with the login's page
//    at `/login`, which hydrates in place with the name kept and the
//    form's error; then it logs in through the form; `/admin`, `/admin/new`
//    and an edit page hydrate in place; it creates a post whose title holds
//    the same script, which shows as text on the admin's list, the home
//    page and its own, and runs nothing; an edit with a blank title, sent
//    by the form's `submit()`, is answered `400` with the editor at
//    `/admin/posts/<id>`, the path it was posted to, which hydrates in
//    place with the body kept and the title's message; it edits the post
//    from there; and deletes it, after which its page is `404`.
// No page logs a console error, a hydration mismatch among them, but the
// one Chromium logs for a document answered `4xx` that the step expects;
// no page throws, and no dialog opens. The summary line says what it did,
// the nodes each page kept, those of the four pages a form was answered
// with at the path it was posted to among them, and the client's size,
// `app.wasm` and `app.js` gzipped, printed and not limited. The server is
// killed at the end, pass or fail. Every wait polls, for 30 seconds at
// most. Playwright is the job's own install, as for `counter.mjs`.
//
// With `--release` after the three, as CI's step of release builds runs it
// with binaryen's `wasm-opt` on `PATH` (spec
// docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md
// §4), it builds with `nova build --ssr --release`, checks that
// `dist/manifest.json` says the client is a release build that `wasm-opt`
// optimized, runs the server from `target/release/`, and its line ends
// with that after the client's size. Without it, nothing changes.

import { spawn } from "node:child_process";
import { cp, mkdir, readFile, rm } from "node:fs/promises";
import { createRequire } from "node:module";
import { dirname, join, relative, resolve, sep } from "node:path";
import { createInterface } from "node:readline";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";

const [nova, work, modules] = process.argv.slice(2, 5).map((p) => resolve(p));
const release = process.argv[5] === "--release";
if (process.argv.length > 6 || (process.argv.length === 6 && !release)) {
  throw new Error("usage: node tests/browser/blog.mjs <nova> <work dir> <dir holding node_modules/playwright> [--release]");
}
const { chromium } = createRequire(join(modules, "noop.js"))("playwright");
const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const LIMIT = 30000;
const PASSWORD = "correct horse battery";
// A script that would set a global and open a dialog, in a tag the parser
// runs, and in one whose error handler would.
const XSS = `<script>window.novaXss = 1; alert("xss")</script><img src=x onerror="window.novaXss = 2; alert('xss')">`;
// The same, short enough for a comment's name, of 60 characters at most.
const NAME_XSS = `<img src=x onerror="window.novaXss=3;alert(3)">`;
const EXE = process.platform === "win32" ? ".exe" : "";
// Where `nova build --ssr` puts the server.
const PROFILE = release ? "release" : "debug";
// The character a browser's parser drops from a page's text.
const NUL = String.fromCharCode(0);

// Runs `command` in `dir` to its end, `input` on its stdin: its status and
// its output. It answers on `close`, once the child's stdout and stderr
// have closed, not on `exit`, which can come while output is still unread.
function run(command, args, dir, env, input = null) {
  return new Promise((done, failed) => {
    const child = spawn(command, args, { cwd: dir, env, stdio: ["pipe", "pipe", "pipe"] });
    let output = "";
    child.stdout.on("data", (chunk) => {
      output += chunk;
    });
    child.stderr.on("data", (chunk) => {
      output += chunk;
    });
    child.on("error", failed);
    child.on("close", (code) => done({ code, output }));
    child.stdin.end(input ?? "");
  });
}

// The server in `dir`: its URL, its stderr so far, and a kill.
async function startServer(dir, env) {
  const exe = join(dir, "target", PROFILE, `fullstack-blog-server${EXE}`);
  const child = spawn(exe, ["--address", "127.0.0.1:0"], { cwd: dir, env, stdio: ["ignore", "pipe", "pipe"] });
  let stderr = "";
  child.stderr.on("data", (chunk) => {
    stderr += chunk;
  });
  const server = { url: null, stderr: () => stderr, kill: () => child.kill() };
  try {
    server.url = await new Promise((found, failed) => {
      const timer = setTimeout(() => failed(new Error("no URL within 30 s")), LIMIT);
      createInterface({ input: child.stdout }).on("line", (line) => {
        const url = /^blog: serving (http:\/\/127\.0\.0\.1:\d+\/)$/.exec(line)?.[1];
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

// Polls `test(arg)` in the page until it holds; a navigation between two
// polls is only a poll that fails.
async function until(page, test, arg, what) {
  const deadline = Date.now() + LIMIT;
  for (;;) {
    try {
      if (await page.evaluate(test, arg)) return;
    } catch {
      // The page is loading.
    }
    if (Date.now() > deadline) {
      const now = await page.evaluate(() => document.querySelector("h1")?.textContent).catch(() => "?");
      throw new Error(`timed out waiting for ${what}; the page's heading reads ${JSON.stringify(now)} at ${page.url()}`);
    }
    await new Promise((later) => setTimeout(later, 100));
  }
}

const state = (page) => page.evaluate(() => document.getElementById("root")?.dataset.state ?? null);
const heading = (page) => page.evaluate(() => document.querySelector("#app h1")?.textContent ?? null);
const hydrated = (page, what) =>
  until(page, () => document.getElementById("root")?.dataset.state === "hydrated", null, `${what} to hydrate`);

// Opens `url`, or runs `navigate`, which leads to it and returns its
// response, with `app.wasm` held back, checks the server's HTML is there
// with the heading `expected`, keeps a handle to each node under `#app`,
// lets `app.wasm` load, waits for hydration, and checks each handle but
// those inside a form's `.actions` is still in the page, in order. Returns
// the response's status and how many nodes were kept.
async function hydratesInPlace(page, url, expected, navigate = () => page.goto(url, { waitUntil: "domcontentloaded" })) {
  let release;
  const held = new Promise((go) => {
    release = go;
  });
  await page.route("**/app.wasm", async (route) => {
    await held;
    // A check that fails while it is held unroutes it first.
    await route.continue().catch(() => {});
  });
  try {
    const response = await navigate();
    if ((await state(page)) !== "server") {
      throw new Error(`${url}: before app.wasm loads, #root's data-state is ${await state(page)}, not "server"`);
    }
    if ((await heading(page)) !== expected) {
      throw new Error(`${url}: the server's HTML shows ${JSON.stringify(await heading(page))}, not ${expected}`);
    }
    // Each form's one submit button, enabled, so the form works without
    // JavaScript.
    const buttons = await page.$$eval("form .actions", (all) =>
      all.map((a) => [...a.querySelectorAll("button")].map((b) => (b.disabled ? "disabled" : "enabled")).join(" ")),
    );
    if (buttons.some((b) => b !== "enabled")) {
      throw new Error(`${url}: before app.wasm loads, the forms' buttons are ${JSON.stringify(buttons)}`);
    }
    // Each node but those inside a form's `.actions`, read while they are
    // all in the page.
    const kept = await page.evaluateHandle(() =>
      window.novaNodes().filter((n) => !n.parentNode?.closest?.(".actions")),
    );
    release();
    await hydrated(page, url);
    const moved = await page.evaluate((nodes) => {
      const gone = nodes.filter((n) => !n.isConnected || !document.getElementById("app").contains(n));
      if (gone.length > 0) return `${gone.length} of the server's ${nodes.length} nodes are gone, the first a ${gone[0].nodeName}`;
      const now = window.novaNodes();
      const at = nodes.map((n) => now.indexOf(n));
      const out = at.findIndex((p, i) => i > 0 && p <= at[i - 1]);
      return out < 0 ? null : `node ${out}, ${nodes[out].nodeName}, moved`;
    }, kept);
    if (moved) throw new Error(`${url} did not hydrate in place: ${moved}`);
    if ((await heading(page)) !== expected) {
      throw new Error(`${url}: the hydrated page shows ${JSON.stringify(await heading(page))}, not ${expected}`);
    }
    const count = await kept.evaluate((nodes) => nodes.length);
    return { status: response.status(), count };
  } finally {
    release();
    await page.unroute("**/app.wasm");
  }
}

// Clicks `selector` and waits for the page it leads to to load and hydrate:
// the response's status.
async function submits(page, selector, what) {
  const [response] = await Promise.all([
    page.waitForNavigation({ waitUntil: "domcontentloaded", timeout: LIMIT }),
    page.click(selector),
  ]);
  await hydrated(page, what);
  return response.status();
}

// Runs `act`, which sends a form, and waits for the page it leads to to
// load: the response, for `hydratesInPlace`'s `navigate`.
async function answered(page, act) {
  const [response] = await Promise.all([
    page.waitForNavigation({ waitUntil: "domcontentloaded", timeout: LIMIT }),
    act(),
  ]);
  return response;
}

// The comment form's submit button is disabled, or not.
const disabled = (page) =>
  page.evaluate(() => {
    const buttons = [...document.querySelectorAll("#comment-form .actions button")];
    return buttons.length === 1 ? buttons[0].disabled : `${buttons.length} buttons`;
  });

const dir = join(work, "fullstack-blog");
const env = { ...process.env, NOVA_HOME: join(work, "nova-home"), NO_COLOR: "1" };
const example = join(repo, "examples", "07-fullstack-blog");
const left = [
  "dist",
  "target",
  join("data", "blog.json"),
  join("data", "blog.json.tmp"),
  join("data", "users.json"),
  join("data", "users.json.tmp"),
  join("data", "secret"),
];
await rm(work, { recursive: true, force: true });
await mkdir(work, { recursive: true });
await cp(example, dir, {
  recursive: true,
  filter: (from) => !left.some((l) => relative(example, from) === l || relative(example, from).startsWith(l + sep)),
});

const flags = release ? ["--ssr", "--release"] : ["--ssr"];
const build = await run(nova, ["build", ...flags], dir, env);
if (build.code !== 0) throw new Error(`nova build ${flags.join(" ")} exited with ${build.code}:\n${build.output}`);
if (release) {
  const manifest = JSON.parse(await readFile(join(dir, "dist", "manifest.json"), "utf8"));
  if (manifest.release !== true || manifest.wasm_opt !== true) {
    throw new Error(`dist/manifest.json says release ${manifest.release} and wasm_opt ${manifest.wasm_opt}, not a release build that wasm-opt optimized`);
  }
}
const exe = join(dir, "target", PROFILE, `fullstack-blog-server${EXE}`);
const set = await run(exe, ["--set-password", "admin", "--iterations", "1000"], dir, env, `${PASSWORD}\n`);
if (set.code !== 0 || set.output.trim() !== "blog: set the password of admin") {
  throw new Error(`--set-password exited with ${set.code}:\n${set.output}`);
}

const server = await startServer(dir, env);
const at = (path) => new URL(path, server.url).href;
let browser = null;
try {
  browser = await chromium.launch();
  const page = await browser.newPage();
  const errors = [];
  const dialogs = [];
  // The documents a step expects a `4xx` for, whose error Chromium logs.
  const expected = new Set();
  page.on("console", (m) => {
    if (m.type() !== "error") return;
    if (expected.has(m.location().url) && /status of 4\d\d/.test(m.text())) return;
    errors.push(`${m.text()} (${m.location().url})`);
  });
  page.on("pageerror", (e) => errors.push(String(e)));
  page.on("dialog", async (d) => {
    dialogs.push(d.message());
    await d.dismiss();
  });
  const clean = (what) => {
    if (errors.length > 0) throw new Error(`${what}: the console has errors: ${errors.join("; ")}`);
    if (dialogs.length > 0) throw new Error(`${what}: a dialog opened: ${dialogs.join("; ")}`);
  };
  await page.addInitScript(() => {
    window.novaNodes = () => {
      const walker = document.createTreeWalker(document.getElementById("app"), NodeFilter.SHOW_ALL);
      const nodes = [];
      while (walker.nextNode()) nodes.push(walker.currentNode);
      return nodes;
    };
  });
  const inert = async (what) => {
    if ((await page.evaluate(() => window.novaXss)) !== undefined) throw new Error(`${what}: the script ran`);
    clean(what);
  };
  const kept = [];
  // The pages a form was answered with at the path it was posted to.
  const answers = [];

  // 1. Every page a visitor sees, in place.
  for (const [path, title, status] of [
    ["/", "Posts", 200],
    ["/posts/1", "Hello from a Nova server", 200],
    ["/login", "Log in", 200],
    ["/nowhere", "Not found", 404],
    ["/posts/99", "Not found", 404],
  ]) {
    if (status !== 200) expected.add(at(path));
    const got = await hydratesInPlace(page, at(path), title);
    if (got.status !== status) throw new Error(`${path} answered ${got.status}, not ${status}`);
    kept.push(`${path} ${got.count}`);
    clean(path);
  }

  // 2. The comment form's checks, then the comment.
  await hydratesInPlace(page, at("/posts/1"), "Hello from a Nova server");
  if ((await disabled(page)) !== true) throw new Error(`a blank comment form's button is ${await disabled(page)}`);
  await page.fill("#comment-form input[name=name]", "Katherine");
  if ((await disabled(page)) !== true) throw new Error("the button is enabled with no comment");
  const messages = () => page.$$eval("#comment-form .message", (ps) => ps.map((p) => p.textContent));
  if ((await messages()).join("|") !== "|Write a comment.") {
    throw new Error(`the messages read ${JSON.stringify(await messages())}`);
  }
  await page.fill("#comment-form textarea[name=text]", "First paragraph.\n\nSecond paragraph.");
  if ((await disabled(page)) !== false) throw new Error("the button is disabled with both fields filled");
  await page.fill("#comment-form input[name=name]", "  ");
  if ((await disabled(page)) !== true) throw new Error("the button is enabled with a blank name");
  if ((await messages())[0] !== "Say who you are.") throw new Error(`the messages read ${await messages()}`);
  await page.fill("#comment-form input[name=name]", "Katherine");
  const posted = await submits(page, "#comment-form button[type=submit]", "the post after a comment");
  if (posted !== 200 || new URL(page.url()).pathname !== "/posts/1" || new URL(page.url()).hash !== "#comments") {
    throw new Error(`the comment led to ${page.url()} with ${posted}`);
  }
  const said = await page.$$eval("#comments .comment", (cs) => cs.map((c) => c.innerText));
  const last = said[said.length - 1] ?? "";
  if (!last.startsWith("Katherine") || !last.includes("First paragraph.") || !last.includes("Second paragraph.")) {
    throw new Error(`the comments read ${JSON.stringify(said)}`);
  }
  clean("a comment");

  // 3. A comment that fails a check, at the path it was posted to.
  await page.fill("#comment-form input[name=name]", "Katherine");
  await page.fill("#comment-form textarea[name=text]", "");
  expected.add(at("/posts/1/comments"));
  const failed = await hydratesInPlace(page, at("/posts/1/comments"), "Hello from a Nova server", () =>
    answered(page, () => page.evaluate(() => document.getElementById("comment-form").submit())),
  );
  if (failed.status !== 400 || new URL(page.url()).pathname !== "/posts/1/comments") {
    throw new Error(`a comment with a blank text got ${failed.status} at ${page.url()}`);
  }
  const name = await page.inputValue("#comment-form input[name=name]");
  if (name !== "Katherine" || (await messages()).join("|") !== "|Write a comment.") {
    throw new Error(`the failed comment's form reads ${JSON.stringify(name)}, ${JSON.stringify(await messages())}`);
  }
  answers.push(`a comment that fails a check, 400 at /posts/1/comments ${failed.count}`);
  clean("a comment that fails a check");

  // 4. A comment holding a NUL.
  await page.evaluate((text) => {
    const field = document.querySelector("#comment-form textarea[name=text]");
    field.value = text;
    field.dispatchEvent(new Event("input", { bubbles: true }));
  }, `one${NUL}two`);
  await submits(page, "#comment-form button[type=submit]", "the post after a comment with a NUL");
  await hydratesInPlace(page, at("/posts/1"), "Hello from a Nova server");
  const nul = await page.$$eval("#comments .comment .body p", (ps) => ps[ps.length - 1].textContent);
  if (nul !== "onetwo") throw new Error(`the comment with a NUL reads ${JSON.stringify(nul)}`);
  clean("a comment with a NUL");

  // 5. A comment holding a script.
  await page.fill("#comment-form input[name=name]", NAME_XSS);
  await page.fill("#comment-form textarea[name=text]", XSS);
  await submits(page, "#comment-form button[type=submit]", "the post after a comment with a script");
  const shown = await page.$$eval("#comments .comment", (cs) => {
    const c = cs[cs.length - 1];
    return { who: c.querySelector("strong").textContent, text: c.querySelector(".body p").textContent };
  });
  if (shown.who !== NAME_XSS || shown.text !== XSS) {
    throw new Error(`the comment with a script reads ${JSON.stringify(shown)}`);
  }
  await page.waitForTimeout(500);
  await inert("a comment with a script");

  // 6. A form without its token, refused at the path it was posted to.
  const before = await page.$$eval("#comments .comment", (cs) => cs.length);
  await page.evaluate(() => document.querySelector("#comment-form input[name=csrf]").remove());
  await page.fill("#comment-form input[name=name]", "Eve");
  await page.fill("#comment-form textarea[name=text]", "No token");
  const forged = await hydratesInPlace(page, at("/posts/1/comments"), "Not sent", () =>
    answered(page, () => page.click("#comment-form button[type=submit]")),
  );
  const refusal = await page.evaluate(() => document.getElementById("refused")?.textContent ?? "");
  if (forged.status !== 403 || new URL(page.url()).pathname !== "/posts/1/comments" || !refusal.includes("expired")) {
    throw new Error(`a comment without its token got ${forged.status} at ${page.url()}: ${JSON.stringify(refusal)}`);
  }
  answers.push(`a comment without its token, 403 at /posts/1/comments ${forged.count}`);
  clean("a comment without its token");
  await hydratesInPlace(page, at("/posts/1"), "Hello from a Nova server");
  const after = await page.$$eval("#comments .comment", (cs) => cs.length);
  if (after !== before) throw new Error(`the refused comment was added: ${before} comments became ${after}`);

  // 7. The admin: a wrong password, answered at `/login`; then log in, and
  // create, edit and delete a post.
  await hydratesInPlace(page, at("/login"), "Log in");
  await page.fill("#login-form input[name=name]", "admin");
  await page.fill("#login-form input[name=password]", "a wrong password");
  expected.add(at("/login"));
  const wrong = await hydratesInPlace(page, at("/login"), "Log in", () =>
    answered(page, () => page.click("#login-form button[type=submit]")),
  );
  const error = await page.evaluate(() => document.getElementById("login-error")?.textContent ?? "");
  const who = await page.inputValue("#login-form input[name=name]");
  if (wrong.status !== 401 || new URL(page.url()).pathname !== "/login" || error !== "Wrong name or password." || who !== "admin") {
    throw new Error(`a wrong password got ${wrong.status} at ${page.url()}, reading ${JSON.stringify(error)} for ${JSON.stringify(who)}`);
  }
  answers.push(`a wrong password, 401 at /login ${wrong.count}`);
  clean("a wrong password");
  await page.fill("#login-form input[name=password]", PASSWORD);
  const logged = await submits(page, "#login-form button[type=submit]", "the admin's page");
  if (logged !== 200 || new URL(page.url()).pathname !== "/admin") {
    throw new Error(`the login led to ${page.url()} with ${logged}`);
  }
  for (const [path, title] of [
    ["/admin", "Admin"],
    ["/admin/new", "A new post"],
    ["/admin/posts/2/edit", "Edit a post"],
  ]) {
    const got = await hydratesInPlace(page, at(path), title);
    if (got.status !== 200) throw new Error(`${path} answered ${got.status}`);
    kept.push(`${path} ${got.count}`);
    clean(path);
  }
  await hydratesInPlace(page, at("/admin"), "Admin");
  await submits(page, "#new-post", "the editor");
  const title = `A script: ${XSS}`;
  await page.fill("#editor input[name=title]", title);
  await page.fill("#editor textarea[name=body]", "Its first paragraph.\n\nIts second.");
  await submits(page, "#editor button[type=submit]", "the admin's page after a new post");
  const listed = await page.$$eval(".row > a:first-child", (as) => as.map((a) => ({ text: a.textContent, href: a.getAttribute("href") })));
  const made = listed.find((l) => l.text === title);
  if (!made) throw new Error(`the admin's list reads ${JSON.stringify(listed)}`);
  await inert("the admin's list");
  const id = made.href.split("/").pop();
  await hydratesInPlace(page, at("/"), "Posts");
  const cards = await page.$$eval(".card h2", (hs) => hs.map((h) => h.textContent));
  if (cards[0] !== title) throw new Error(`the home page's first post is ${JSON.stringify(cards[0])}`);
  await inert("the home page");
  await hydratesInPlace(page, at(made.href), title);
  await inert("the new post's page");

  await hydratesInPlace(page, at(`/admin/posts/${id}/edit`), "Edit a post");
  // An edit that fails a check, at the path it was posted to.
  await page.fill("#editor input[name=title]", "");
  expected.add(at(`/admin/posts/${id}`));
  const refused = await hydratesInPlace(page, at(`/admin/posts/${id}`), "Edit a post", () =>
    answered(page, () => page.evaluate(() => document.getElementById("editor").submit())),
  );
  if (refused.status !== 400 || new URL(page.url()).pathname !== `/admin/posts/${id}`) {
    throw new Error(`an edit with a blank title got ${refused.status} at ${page.url()}`);
  }
  const body = await page.inputValue("#editor textarea[name=body]");
  const notes = await page.$$eval("#editor .message", (ps) => ps.map((p) => p.textContent));
  if (body !== "Its first paragraph.\n\nIts second." || notes.join("|") !== "Give the post a title.|") {
    throw new Error(`the failed edit's form reads ${JSON.stringify(body)}, ${JSON.stringify(notes)}`);
  }
  answers.push(`an edit that fails a check, 400 at /admin/posts/${id} ${refused.count}`);
  clean("an edit that fails a check");
  await page.fill("#editor input[name=title]", "An edited post");
  await submits(page, "#editor button[type=submit]", "the admin's page after an edit");
  const edited = await page.$$eval(".row > a:first-child", (as) => as.map((a) => a.textContent));
  if (!edited.includes("An edited post") || edited.includes(title)) {
    throw new Error(`after the edit the admin's list reads ${JSON.stringify(edited)}`);
  }
  await hydratesInPlace(page, at(made.href), "An edited post");

  await hydratesInPlace(page, at("/admin"), "Admin");
  await submits(page, `form[action="/admin/posts/${id}/delete"] button[type=submit]`, "the admin's page after a delete");
  const remaining = await page.$$eval(".row > a:first-child", (as) => as.map((a) => a.textContent));
  if (remaining.includes("An edited post")) throw new Error(`the deleted post is still listed: ${remaining}`);
  expected.add(at(made.href));
  const deleted = await hydratesInPlace(page, at(made.href), "Not found");
  if (deleted.status !== 404) throw new Error(`the deleted post's page answered ${deleted.status}`);
  clean("the admin's pages");

  // The client's size, printed and not limited, measured as the size test
  // measures 06's: each file gzipped at the best level.
  let gzipped = 0;
  for (const name of ["app.wasm", "app.js"]) {
    gzipped += gzipSync(await readFile(join(dir, "dist", name)), { level: 9 }).length;
  }

  console.log(
    `blog: every page hydrates in place (nodes kept: ${kept.join(", ")}), and so does each form's answer at the path it was posted to (nodes kept: ${answers.join("; ")}), the comment form's button is disabled while a field is blank and the comment posts, a comment holding a NUL comes back without it, a script in a comment and in a title stays text, a form without its token gets 403, and the admin logs in and creates, edits and deletes a post; the client is ${gzipped} bytes gzipped${release ? ", from a release build that wasm-opt optimized" : ""}`,
  );
} catch (e) {
  console.error(`the server's stderr:\n${server.stderr()}`);
  throw e;
} finally {
  await browser?.close();
  server.kill();
}
