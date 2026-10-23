// The Phase 4 gate's measurement (spec
// docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md
// §3, the user's decisions 1 and 2), run by CI's `browser` job last, after
// the release builds and the rules' test:
//
//     node tests/browser/lighthouse.mjs <nova> <work dir> <dir holding node_modules/playwright and node_modules/lighthouse>
//
// It copies `examples/06-counter-spa` and `examples/07-fullstack-blog` into
// the work directory, emptied first, leaving out what a build or a run there
// may have left, as `blog.mjs` does, and builds each there with `NOVA_HOME`
// beside them. 06 is built with `nova build --target wasm`, a debug build,
// as the size tests and `spa.mjs` measure it, and its `dist/` is served by a
// static server on `127.0.0.1:0`, as `counter.mjs` serves one. 07 is built
// with `nova build --ssr`; the admin's password is set through
// `fullstack-blog-server --set-password admin --iterations 1000`, and the
// server is started on `--address 127.0.0.1:0`, its port read from the line
// `blog: serving http://127.0.0.1:<port>/`, as `blog.mjs` does.
//
// Lighthouse's CLI, from the npm directory's `lighthouse` (pinned by the
// job to 13.5.0), then runs three times on each of 06's `/` and 07's `/`,
// `/posts/1` and `/login`: the mobile preset, its default, with its
// simulated throttling; the Performance category alone; a JSON report
// written into the work directory; and Chromium headless without its
// sandbox, the Chromium Playwright installed, named by `CHROME_PATH`. Each
// page's line gives its three scores and their median, the median of each
// main metric, and the runs' benchmark indexes, also when a run failed. A
// run fails when Lighthouse exits other than 0, is stopped after three
// minutes, or leaves no report, or a report with a runtime error or no
// score. Every run runs; then the script fails when a run failed, a page
// had other than three runs, or a page's median is under 95. Its last line
// begins `lighthouse:` and says which, and, when every page passed, the
// versions of Lighthouse, Chromium and Node; a failure before the verdict,
// such as a build that fails, gets a `lighthouse: failed` line too, with
// its reason: for a build or a server, the first line of its stderr that
// begins `error`, or else its last, cut to 200 characters, its whole
// output going to stderr. Lighthouse 13 needs Node 22.19 or later, which
// the script checks first. Both servers are stopped at the end, pass or
// fail. Lighthouse and Playwright are the job's own install: nothing in
// `cargo test` or `nova` installs a package.
//
// The pages, the number of runs, what a failed run is, the median and the
// verdict are `lighthouse-rules.mjs`'s, which `lighthouse.test.mjs` tests
// without Lighthouse; this script runs Lighthouse and applies them.

import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { createServer } from "node:http";
import { cp, mkdir, readFile, rm } from "node:fs/promises";
import { createRequire } from "node:module";
import { dirname, extname, join, relative, resolve, sep } from "node:path";
import { createInterface } from "node:readline";
import { fileURLToPath } from "node:url";

import { PASS, RUNS, judge, pages, runOf } from "./lighthouse-rules.mjs";

const [nova, work, modules] = process.argv.slice(2).map((p) => resolve(p));
const lighthouse = join(modules, "node_modules", "lighthouse", "cli", "index.js");
const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const LIMIT = 30000;
// A run takes about 11 s on a developer's machine; one still going after
// this long is stopped, and fails.
const RUN_LIMIT = 180000;
const PASSWORD = "correct horse battery";
const EXE = process.platform === "win32" ? ".exe" : "";

const types = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".json": "application/json",
  ".wasm": "application/wasm",
  ".css": "text/css",
};

// Runs `command` in `dir` to its end, `input` on its stdin: its status, its
// output and its stderr alone, once its stdout and stderr have closed. A
// child that exits before it reads its stdin breaks the pipe; the write's
// EPIPE is ignored, since the status says how the run went.
function run(command, args, dir, env, input = null) {
  return new Promise((done, failed) => {
    const child = spawn(command, args, { cwd: dir, env, stdio: ["pipe", "pipe", "pipe"] });
    let output = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => {
      output += chunk;
    });
    child.stderr.on("data", (chunk) => {
      output += chunk;
      stderr += chunk;
    });
    child.on("error", failed);
    child.on("close", (code) => done({ code, output, stderr }));
    child.stdin.on("error", () => {});
    child.stdin.end(input ?? "");
  });
}

// The reason a build or a server failed, from its stderr (or its output,
// when its stderr is empty): the first line that begins `error`, which
// names the fault where `nova` and cargo print one, or else the last line
// that is not empty.
function reason(stderr, output) {
  const lines = (stderr.trim() === "" ? output : stderr)
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line !== "");
  return lines.find((line) => /^error/i.test(line)) ?? lines.at(-1) ?? "no output";
}

// A step that failed: its whole output to stderr, and an error naming it
// and its reason, for the `lighthouse:` line.
function failure(step, { output, stderr }) {
  console.error(`${step}:\n${output}`);
  return new Error(`${step}: ${reason(stderr, output)}`);
}

// `text` as one line of at most 200 characters, its lines joined by " / ".
function oneLine(text) {
  const line = String(text)
    .split("\n")
    .map((part) => part.trim())
    .filter((part) => part !== "")
    .join(" / ");
  return line.length > 200 ? `${line.slice(0, 197)}...` : line;
}

// `example` copied to `dir`, without the paths in `left`.
async function copy(example, dir, left) {
  const from = join(repo, "examples", example);
  await cp(from, dir, {
    recursive: true,
    filter: (path) => !left.some((l) => relative(from, path) === l || relative(from, path).startsWith(l + sep)),
  });
}

// `dist/`'s files, and nothing outside it, on 127.0.0.1:0: the server and
// its URL.
async function serve(dist) {
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
  return { server, url: `http://127.0.0.1:${server.address().port}/` };
}

// The blog's server in `dir`: its URL, its stderr so far, and a kill.
async function startServer(dir, env) {
  const exe = join(dir, "target", "debug", `fullstack-blog-server${EXE}`);
  const child = spawn(exe, ["--address", "127.0.0.1:0"], { cwd: dir, env, stdio: ["ignore", "pipe", "pipe"] });
  let stderr = "";
  child.stderr.on("data", (chunk) => {
    stderr += chunk;
  });
  const server = { url: null, stderr: () => stderr, kill: () => child.kill() };
  try {
    server.url = await new Promise((found, failed) => {
      const timer = setTimeout(() => failed(new Error("the blog's server gave no URL within 30 s")), LIMIT);
      createInterface({ input: child.stdout }).on("line", (line) => {
        const url = /^blog: serving (http:\/\/127\.0\.0\.1:\d+\/)$/.exec(line)?.[1];
        if (url) {
          clearTimeout(timer);
          found(url);
        }
      });
      // "close", not "exit": its stderr has been read to the end by then.
      child.on("close", (code) => failed(new Error(`the blog's server exited with ${code}`)));
    });
  } catch (e) {
    child.kill();
    throw failure(e.message, { output: stderr, stderr });
  }
  return server;
}

// One run of Lighthouse's CLI on `url`, its report written to `report`:
// its exit status, or the signal that stopped it, and its output. One that
// runs past `RUN_LIMIT` gets SIGINT, on which Lighthouse closes its
// Chromium, and then SIGKILL.
function lighthouseRun(url, report, chrome) {
  const args = [
    lighthouse,
    url,
    "--only-categories=performance",
    "--output=json",
    `--output-path=${report}`,
    "--quiet",
    "--chrome-flags=--headless=new --no-sandbox",
  ];
  return new Promise((done, failed) => {
    const child = spawn(process.execPath, args, {
      env: { ...process.env, CHROME_PATH: chrome },
      stdio: ["ignore", "pipe", "pipe"],
    });
    let output = "";
    child.stdout.on("data", (chunk) => {
      output += chunk;
    });
    child.stderr.on("data", (chunk) => {
      output += chunk;
    });
    let kill = null;
    const timer = setTimeout(() => {
      output += `\nstopped after ${RUN_LIMIT / 1000} s`;
      child.kill("SIGINT");
      kill = setTimeout(() => child.kill("SIGKILL"), 10000);
    }, RUN_LIMIT);
    child.on("error", failed);
    child.on("close", (code, signal) => {
      clearTimeout(timer);
      clearTimeout(kill);
      done({ code, signal, output });
    });
  });
}

// `RUNS` runs on `url`, each report at `<reports>/<label>-<n>.json`: one
// entry per run, `runOf`'s, with its score out of 100, its metrics, its
// benchmark index and its warnings, or why it failed.
async function measure(label, url, chrome, reports) {
  const runs = [];
  for (let n = 1; n <= RUNS; n++) {
    const report = join(reports, `${label}-${n}.json`);
    const done = await lighthouseRun(url, report, chrome);
    let lhr = null;
    try {
      lhr = JSON.parse(await readFile(report, "utf8"));
    } catch {
      // No report, or not one that parses: the run failed.
    }
    const run = runOf(done, lhr, report);
    if (run.failure !== null) {
      console.error(`${label}, run ${n}: ${run.failure}; Lighthouse's output:\n${done.output}`);
    }
    runs.push(run);
  }
  return runs;
}

// Every failure, a thrown one too (a build, a server that does not start,
// Node below the floor), ends with one `lighthouse:` line that says why,
// and a status of 1; both servers are stopped whatever happens.
let statics = null;
let server = null;
try {
  // Lighthouse 13 needs Node 22.19 or later, by its `engines`.
  const [major, minor] = process.versions.node.split(".").map(Number);
  if (major < 22 || (major === 22 && minor < 19)) {
    throw new Error(`Lighthouse 13 needs Node 22.19 or later, and this is Node ${process.versions.node}`);
  }
  const { chromium } = createRequire(join(modules, "noop.js"))("playwright");
  const chrome = chromium.executablePath();
  if (!existsSync(chrome)) throw new Error(`Playwright's Chromium is not at ${chrome}`);
  if (!existsSync(lighthouse)) throw new Error(`Lighthouse's CLI is not at ${lighthouse}`);

  const env = { ...process.env, NOVA_HOME: join(work, "nova-home"), NO_COLOR: "1" };
  const spa = join(work, "counter-spa");
  const blog = join(work, "fullstack-blog");
  const reports = join(work, "reports");
  await rm(work, { recursive: true, force: true });
  await mkdir(reports, { recursive: true });
  await copy("06-counter-spa", spa, ["dist", "target"]);
  await copy("07-fullstack-blog", blog, [
    "dist",
    "target",
    join("data", "blog.json"),
    join("data", "blog.json.tmp"),
    join("data", "users.json"),
    join("data", "users.json.tmp"),
    join("data", "secret"),
  ]);

  const built = await run(nova, ["build", "--target", "wasm"], spa, env);
  if (built.code !== 0) throw failure(`nova build --target wasm of 06 exited with ${built.code}`, built);
  const ssr = await run(nova, ["build", "--ssr"], blog, env);
  if (ssr.code !== 0) throw failure(`nova build --ssr of 07 exited with ${ssr.code}`, ssr);
  const exe = join(blog, "target", "debug", `fullstack-blog-server${EXE}`);
  const set = await run(exe, ["--set-password", "admin", "--iterations", "1000"], blog, env, `${PASSWORD}\n`);
  if (set.code !== 0 || set.output.trim() !== "blog: set the password of admin") {
    throw failure(`--set-password exited with ${set.code}`, set);
  }

  statics = await serve(join(spa, "dist"));
  server = await startServer(blog, env);
  const failures = [];
  const medians = [];
  let about = null;
  for (const [name, label, url] of pages(statics.url, server.url)) {
    const runs = await measure(label, url, chrome, reports);
    const page = judge(name, runs);
    console.log(page.line);
    failures.push(...page.failures);
    medians.push(`${name} ${page.score ?? "none"}`);
    const first = runs.find((r) => r.failure === null);
    about ??= first ? `Lighthouse ${first.version}, ${first.browser}, Node ${process.versions.node}` : null;
  }
  const what = `06's / and 07's /, /posts/1 and /login, ${RUNS} runs each, mobile`;
  if (failures.length > 0) {
    console.log(`lighthouse: failed, ${what}: ${failures.join("; ")}`);
    process.exitCode = 1;
  } else {
    console.log(`lighthouse: ${what}, every median ${PASS} or more (${medians.join(", ")}); ${about}`);
  }
} catch (e) {
  if (server) console.error(`the blog server's stderr:\n${server.stderr()}`);
  console.error(e);
  console.log(`lighthouse: failed, before its verdict: ${oneLine(e?.message ?? e)}`);
  process.exitCode = 1;
} finally {
  server?.kill();
  statics?.server.close();
}
