// `run`, which `lighthouse.mjs` builds the examples and sets the blog's
// password with, in a module of its own with no side effects, so that
// `lighthouse.test.mjs` can test it under `node --test`, with Node's
// built-ins alone (spec
// docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md
// §3).

import { spawn } from "node:child_process";

// Runs `command` in `dir` to its end, `input` on its stdin: its status, its
// output and its stderr alone, once its stdout and stderr have closed. A
// child that exits before it reads its stdin breaks the pipe; the write's
// EPIPE is ignored, since the status says how the run went.
export function run(command, args, dir, env, input = null) {
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
