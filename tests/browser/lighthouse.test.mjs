// The Lighthouse gate's rules, `lighthouse-rules.mjs`, tested without
// Lighthouse or a browser (spec
// docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md
// §3, the user's decisions 1 and 2): the four pages and three runs each, the
// median of three, and the verdict over a page's runs, a run that failed or
// gave no score, and runs other than three in number, among them. Every
// page here scores 100 or nearly, so `lighthouse.mjs`'s own runs would not
// notice a wrong median or a dropped failure; these do. And
// `lighthouse-run.mjs`'s `run`, which `lighthouse.mjs` builds with, given
// a child that exits before it reads its stdin. Node's own test runner
// runs them, as CI's `browser` job does before the measurement:
//
//     node --test tests/browser/lighthouse.test.mjs

import assert from "node:assert/strict";
import { dirname } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

import { PASS, RUNS, judge, median, pages, runOf } from "./lighthouse-rules.mjs";
import { run as runChild } from "./lighthouse-run.mjs";

// A run that scored `score`, as `runOf` returns one.
function run(score) {
  return {
    failure: null,
    score,
    metrics: [901.2, 1651.7, 3, 0.0007, 901.2],
    bench: 2312.5,
    warnings: [],
    version: "13.5.0",
    browser: "HeadlessChrome/141.0.0.0",
  };
}

// A report as Lighthouse writes one, its Performance score `score`.
function report(score) {
  return {
    lighthouseVersion: "13.5.0",
    categories: { performance: { score } },
    audits: {
      "first-contentful-paint": { numericValue: 901.2 },
      "largest-contentful-paint": { numericValue: 1651.7 },
      "total-blocking-time": { numericValue: 3 },
      "cumulative-layout-shift": { numericValue: 0.0007 },
      "speed-index": { numericValue: 901.2 },
    },
    environment: {
      benchmarkIndex: 2312.5,
      hostUserAgent: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) HeadlessChrome/141.0.0.0 Safari/537.36",
    },
    runWarnings: [],
  };
}

const exited = (code) => ({ code, signal: null });

test("four pages, 06's / and 07's /, /posts/1 and /login, three runs each, 95 to pass", () => {
  assert.deepEqual(pages("http://127.0.0.1:41001/", "http://127.0.0.1:41002/"), [
    ["06 /", "06", "http://127.0.0.1:41001/"],
    ["07 /", "07-home", "http://127.0.0.1:41002/"],
    ["07 /posts/1", "07-post", "http://127.0.0.1:41002/posts/1"],
    ["07 /login", "07-login", "http://127.0.0.1:41002/login"],
  ]);
  assert.equal(RUNS, 3);
  assert.equal(PASS, 95);
});

test("the median of three is the middle score, in any order, and leaves them as they are", () => {
  const scores = [93, 100, 99];
  assert.equal(median(scores), 99);
  assert.deepEqual(scores, [93, 100, 99]);
  assert.equal(median([100, 93, 100]), 100);
  assert.equal(median([94, 90, 100]), 94);
});

test("a page passes on a median of 95 or more, a low run among its three", () => {
  const page = judge("07 /", [run(93), run(100), run(99)]);
  assert.deepEqual(page.failures, []);
  assert.equal(page.score, 99);
  assert.equal(
    page.line,
    "07 /: scores 93, 100, 99, median 99; medians FCP 901 ms, LCP 1652 ms, TBT 3 ms, CLS 0.001, SI 901 ms; benchmark index 2313, 2313, 2313",
  );
  assert.deepEqual(judge("06 /", [run(95), run(100), run(95)]).failures, []);
});

test("a median under 95 fails the page", () => {
  const page = judge("07 /login", [run(100), run(94), run(90)]);
  assert.equal(page.score, 94);
  assert.deepEqual(page.failures, ["07 /login's median is 94, under 95"]);
});

test("a failed run fails the page, whatever the other two scored, its line keeping the benchmark indexes", () => {
  const page = judge("07 /posts/1", [run(100), { failure: "Lighthouse exited with 1" }, run(100)]);
  assert.deepEqual(page.failures, ["07 /posts/1's run 2 failed (Lighthouse exited with 1)"]);
  assert.equal(page.score, null);
  assert.equal(page.line, "07 /posts/1: scores 100, failed, 100; benchmark index 2313, none, 2313");
  const errored = judge("07 /", [run(100), { failure: "NO_FCP: no paint", bench: 811.4 }, run(100)]);
  assert.equal(errored.line, "07 /: scores 100, failed, 100; benchmark index 2313, 811, 2313");
});

test("a run that gave no score fails the page, and so does a page with no runs", () => {
  const page = judge("06 /", [run(100), run(100), { failure: null }]);
  assert.deepEqual(page.failures, ["06 /'s run 3 failed (no Performance score)"]);
  assert.equal(page.score, null);
  assert.deepEqual(judge("06 /", []).failures, ["06 / had 0 runs, not 3", "06 / has no median"]);
});

test("two runs, or four, fail the page, however high they scored", () => {
  const two = judge("07 /login", [run(90), run(100)]);
  assert.deepEqual(two.failures, ["07 /login had 2 runs, not 3"]);
  assert.match(two.line, /^07 \/login: scores 90, 100, median 100; /);
  const four = judge("07 /login", [run(100), run(100), run(100), run(100)]);
  assert.deepEqual(four.failures, ["07 /login had 4 runs, not 3"]);
});

test("a run fails when Lighthouse exits other than 0, leaves no report, or reports an error or no score", () => {
  const ok = runOf(exited(0), report(0.99), "r.json");
  assert.equal(ok.failure, null);
  assert.equal(ok.score, 99);
  assert.deepEqual(ok.metrics, [901.2, 1651.7, 3, 0.0007, 901.2]);
  assert.equal(ok.bench, 2312.5);
  assert.equal(ok.version, "13.5.0");
  assert.equal(ok.browser, "HeadlessChrome/141.0.0.0");
  assert.deepEqual(runOf(exited(1), report(1), "r.json"), { failure: "Lighthouse exited with 1", bench: 2312.5 });
  // 130 is Lighthouse's own exit on SIGINT, which the 180-second limit sends.
  assert.deepEqual(runOf(exited(130), report(1), "r.json"), {
    failure: "Lighthouse exited with 130",
    bench: 2312.5,
  });
  assert.deepEqual(runOf({ code: null, signal: "SIGKILL" }, null, "r.json"), {
    failure: "Lighthouse exited with SIGKILL",
    bench: undefined,
  });
  assert.deepEqual(runOf(exited(0), null, "r.json"), { failure: "no report at r.json", bench: undefined });
  const errored = { ...report(null), runtimeError: { code: "ERRORED_DOCUMENT_REQUEST", message: "(Status code: 404)" } };
  assert.deepEqual(runOf(exited(0), errored, "r.json"), {
    failure: "ERRORED_DOCUMENT_REQUEST: (Status code: 404)",
    bench: 2312.5,
  });
  assert.deepEqual(runOf(exited(0), report(null), "r.json"), { failure: "no Performance score", bench: 2312.5 });
  const other = { ...report(1), categories: { accessibility: { score: 1 } } };
  assert.deepEqual(runOf(exited(0), other, "r.json"), { failure: "no Performance score", bench: 2312.5 });
});

test("run gives a child's status when the child exits before it reads its stdin", async () => {
  // 1 MiB is more than a pipe holds, so the write is still under way when
  // the child exits, and fails with EPIPE, which `run` ignores.
  const here = dirname(fileURLToPath(import.meta.url));
  const done = await runChild(process.execPath, ["-e", "process.exit(3)"], here, process.env, "x".repeat(1 << 20));
  assert.equal(done.code, 3);
});
