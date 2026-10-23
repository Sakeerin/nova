// The rules of the Phase 4 gate's measurement (spec
// docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md
// §3, the user's decisions 1 and 2): the pages Lighthouse measures, how
// many runs each gets, what a run that failed is, the median, and the
// verdict over a page's runs. `lighthouse.mjs` applies them to Lighthouse's
// runs, and `lighthouse.test.mjs` tests them without Lighthouse or a
// browser, under `node --test`, as CI's `browser` job runs it before the
// measurement. They hold no state and touch nothing outside their
// arguments.

// The runs each page gets, and the median a page needs to pass.
export const RUNS = 3;
export const PASS = 95;

// The main metrics of a report, by their audits, each with its unit.
export const METRICS = [
  ["FCP", "first-contentful-paint", "ms"],
  ["LCP", "largest-contentful-paint", "ms"],
  ["TBT", "total-blocking-time", "ms"],
  ["CLS", "cumulative-layout-shift", ""],
  ["SI", "speed-index", "ms"],
];

// The pages Lighthouse measures, each `[name, label, url]`: 06's `/`, on
// the static server at `spa`, and 07's `/`, `/posts/1` and `/login`, on its
// own server at `blog`. `label` names the page's reports.
export function pages(spa, blog) {
  return [
    ["06 /", "06", new URL("/", spa).href],
    ["07 /", "07-home", new URL("/", blog).href],
    ["07 /posts/1", "07-post", new URL("/posts/1", blog).href],
    ["07 /login", "07-login", new URL("/login", blog).href],
  ];
}

// The middle value once sorted, which for three runs is their median; the
// values themselves are left as they are.
export const median = (values) => [...values].sort((a, b) => a - b)[Math.floor(values.length / 2)];

// One run, from Lighthouse's exit (`code`, or the `signal` that stopped it)
// and its report at `report`, parsed, or null when there was none or it did
// not parse: `{ failure, bench }` when the run failed, with the report's
// benchmark index if it gave one, or its score out of 100, its metrics, its
// benchmark index, its warnings and the versions it ran on. A run fails
// when Lighthouse exits other than 0, leaves no report, or a report with a
// runtime error or no Performance score.
export function runOf(done, lhr, report) {
  const score = lhr?.categories?.performance?.score;
  let failure = null;
  if (done.code !== 0) {
    failure = `Lighthouse exited with ${done.code ?? done.signal}`;
  } else if (lhr === null) {
    failure = `no report at ${report}`;
  } else if (lhr.runtimeError) {
    failure = `${lhr.runtimeError.code}: ${lhr.runtimeError.message}`;
  } else if (typeof score !== "number") {
    failure = "no Performance score";
  }
  if (failure !== null) return { failure, bench: lhr?.environment?.benchmarkIndex };
  return {
    failure,
    score: Math.round(score * 100),
    metrics: METRICS.map(([, id]) => lhr.audits?.[id]?.numericValue),
    bench: lhr.environment?.benchmarkIndex,
    warnings: lhr.runWarnings ?? [],
    version: lhr.lighthouseVersion,
    browser: /HeadlessChrome\/[\d.]+|Chrome\/[\d.]+/.exec(lhr.environment?.hostUserAgent ?? "")?.[0],
  };
}

// A page's verdict over its runs: its line, its failures, and its median
// (null when a run failed). A run that failed, or that gave no score,
// fails the page; so do runs other than `RUNS` in number, a median under
// `PASS`, and one that is not a number. The line gives the runs' scores
// and, when every run gave one, their median and the median of each
// metric; then the runs' benchmark indexes, Lighthouse's measure of the
// host's speed, which tell a slow runner from a slow page; then any
// warnings.
export function judge(name, runs) {
  const failures = runs.flatMap((r, i) => {
    if (r.failure != null) return [`${name}'s run ${i + 1} failed (${r.failure})`];
    if (typeof r.score !== "number") return [`${name}'s run ${i + 1} failed (no Performance score)`];
    return [];
  });
  const scored = (r) => r.failure == null && typeof r.score === "number";
  let line = `${name}: scores ${runs.map((r) => (scored(r) ? r.score : "failed")).join(", ")}`;
  const bench = runs.map((r) => (typeof r.bench === "number" ? Math.round(r.bench) : "none")).join(", ");
  if (failures.length > 0) return { line: `${line}; benchmark index ${bench}`, failures, score: null };
  const score = median(runs.map((r) => r.score));
  if (runs.length !== RUNS) failures.push(`${name} had ${runs.length} runs, not ${RUNS}`);
  if (typeof score !== "number") failures.push(`${name} has no median`);
  if (score < PASS) failures.push(`${name}'s median is ${score}, under ${PASS}`);
  const metrics = METRICS.map(([short, , unit], i) => {
    const value = median(runs.map((r) => r.metrics?.[i]));
    if (typeof value !== "number") return `${short} none`;
    return unit === "" ? `${short} ${value.toFixed(3)}` : `${short} ${Math.round(value)} ${unit}`;
  });
  line += `, median ${score}; medians ${metrics.join(", ")}; benchmark index ${bench}`;
  const warnings = [...new Set(runs.flatMap((r) => r.warnings ?? []))];
  if (warnings.length > 0) line += `; warnings: ${warnings.join(" | ")}`;
  return { line, failures, score };
}
