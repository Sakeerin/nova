// Hydrate other views than the server rendered (spec
// docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
// §4.3, §9.3). `server.txt`, beside this file, is what
// `hydrate_mismatch/src/server.nova` printed natively: each root's id and
// HTML. The client reports each root's first difference with
// `console.error`, which the fake DOM prints in order, and builds the
// client's view there instead. This prints each root then, and says whether
// any of the server's nodes is left; then it clicks the server's button,
// whose handler the rebuild removed, and the client's, a turn apart.
import { readFileSync } from "node:fs";
import { install } from "./fake_dom.mjs";
const win = install();
const roots = {};
for (const line of readFileSync(new URL("./server.txt", import.meta.url), "utf8").split("\n")) {
  const at = line.indexOf(" ");
  if (at <= 0) continue;
  const root = document.createElement("div");
  root.setAttribute("id", line.slice(0, at));
  root.innerHTML = line.slice(at + 1);
  document.body.appendChild(root);
  roots[line.slice(0, at)] = root;
}
// Each node under `root`.
function nodes(root) {
  const out = [];
  const visit = (n) => {
    for (const c of n.childNodes) {
      out.push(c);
      visit(c);
    }
  };
  visit(root);
  return out;
}
const before = {};
for (const name in roots) before[name] = nodes(roots[name]);
const serverButton = roots.stops.querySelector("#go");
const turn = () => new Promise((r) => setTimeout(r, 0));

const { default: init } = await import("./dist/app.js");
await init();
for (const name in roots) {
  const left = before[name].filter((n) => roots[name].contains(n)).length;
  console.log(`${name}: ${roots[name].innerHTML} (server nodes left: ${left})`);
}
serverButton.click();
await turn();
console.log("clicked the server's button");
roots.stops.querySelector("#go").click();
await turn();
console.log(`stops: ${roots.stops.innerHTML}`);
console.log(`console errors: ${win.errors.length}`);
