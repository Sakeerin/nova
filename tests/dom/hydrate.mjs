// Hydrate what the server rendered (spec
// docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
// §9.3). `server.txt`, beside this file, is what `hydrate/src/server.nova`
// printed natively: each root's id and HTML, and the guests' JSON. The page
// gets each root with that HTML, the data's `script` element after the
// guests' root, past whitespace and a comment, which the client steps over,
// and six roots the server left empty, one of them holding whitespace and
// one a form feed. The guests' view holds an element whose id is
// `nova-data` too. Three roots have, after them, an element that is not
// the data though it holds other guests as JSON: after `decoy`, which the
// server filled, a `div` with the data's id and type; after `untyped`,
// empty, a `script` with the data's id and no type; and after `behind`,
// empty, a plain `p` and then a `script` with the data's id and type, which
// the client never reaches, since the first element after a root is the
// data or there is none. Each hydrates with its fallback. Two more roots
// hold a text of 70,000 characters, fixed in `long` and dynamic in
// `longer`, which the fake DOM's parser, as Chromium's, splits into two
// nodes, and hydration joins again into the first. Once the client has
// hydrated them, this prints each root, a run of `0123456789` by its count;
// says which of the server's nodes are still there, which nodes are new,
// and how many the parser split off a text; and reads the counter's
// textarea, whose fixed value the server wrote as an attribute, and the
// lengths of the long roots' text nodes. Then it clicks, a turn apart,
// printing the page after each.
import { readFileSync } from "node:fs";
import { install } from "./fake_dom.mjs";
const win = install();
const server = new Map();
for (const line of readFileSync(new URL("./server.txt", import.meta.url), "utf8").split("\n")) {
  const at = line.indexOf(" ");
  if (at > 0) server.set(line.slice(0, at), line.slice(at + 1));
}
// Guests that are not the data, which no root may show.
const other = JSON.stringify({ title: "Not the data", names: ["Eve"] });
const roots = {};
for (const name of ["counter", "lists", "texts", "data", "filled", "fallback", "decoy", "untyped", "behind", "empty", "blank", "feed", "long", "longer"]) {
  const root = document.createElement("div");
  root.setAttribute("id", name);
  if (server.has(name)) root.innerHTML = server.get(name);
  document.body.appendChild(root);
  roots[name] = root;
  if (name === "data") {
    document.body.appendChild(document.createTextNode("\n  "));
    document.body.appendChild(document.createComment(" the guests "));
    document.body.appendChild(document.createTextNode("\n  "));
    const data = document.createElement("script");
    data.setAttribute("type", "application/json");
    data.setAttribute("id", "nova-data");
    data.textContent = server.get("nova-data");
    document.body.appendChild(data);
  }
  if (name === "decoy" || name === "untyped") {
    const next = document.createElement(name === "decoy" ? "div" : "script");
    if (name === "decoy") next.setAttribute("type", "application/json");
    next.setAttribute("id", "nova-data");
    next.textContent = other;
    document.body.appendChild(next);
  }
  if (name === "behind") {
    const plain = document.createElement("p");
    plain.textContent = "not the data";
    document.body.appendChild(plain);
    const late = document.createElement("script");
    late.setAttribute("type", "application/json");
    late.setAttribute("id", "nova-data");
    late.textContent = other;
    document.body.appendChild(late);
  }
}
roots.blank.appendChild(document.createTextNode(" \n\t "));
roots.feed.appendChild(document.createTextNode("\f\n"));

// Each node under `root`, before its children.
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
// The text nodes the parser split off a text, each just after a text node.
const split = {};
for (const name in roots) {
  before[name] = nodes(roots[name]);
  split[name] = before[name].filter((n) => {
    const siblings = n.parentNode.childNodes;
    const at = siblings.indexOf(n);
    return n.nodeName === "#text" && at > 0 && siblings[at - 1].nodeName === "#text";
  });
}
// The lengths of the text nodes in a root's first element.
const lengths = (name) =>
  roots[name].firstChild.childNodes
    .filter((n) => n.nodeName === "#text")
    .map((n) => n.data.length)
    .join(", ");
const parsed = { long: lengths("long"), longer: lengths("longer") };
// The counter's textarea: the server wrote its value as an attribute,
// which a textarea ignores, so until the client sets the property its value
// is its text, "".
const note = roots.counter.querySelector("#note");
console.log(`counter: before hydration, the textarea's value is "${note.value}"`);

// A string to print on one line, and short: a run of `0123456789` ten
// times or more is written as its count.
const line = (s) =>
  s
    .replace(/\n/g, "\\n")
    .replace(/\r/g, "\\r")
    .replace(/\t/g, "\\t")
    .replace(/\f/g, "\\f")
    .replace(/(?:0123456789){10,}/g, (m) => `(0123456789 x ${m.length / 10})`);
const describe = (n) =>
  n.nodeName === "#text" ? `text "${line(n.data)}"` : n.nodeName === "#comment" ? `<!--${n.data}-->` : `<${n.localName}>`;
const show = (name) => console.log(`${name}: ${line(roots[name].innerHTML)}`);
// How many of the server's nodes under the root are there still, whether in
// their order, and the nodes the client made; and, where the parser split a
// text, how many nodes it split off, which hydration joins into the node
// before them, and how many of those are in the page still.
function kept(name) {
  const after = nodes(roots[name]);
  const server = before[name].filter((n) => !split[name].includes(n));
  const stayed = after.filter((n) => server.includes(n));
  const made = after.filter((n) => !before[name].includes(n));
  const order = stayed.every((n, i) => n === server[i]) ? "in order" : "out of order";
  const news = made.length === 0 ? "none" : made.map(describe).join(", ");
  const left = split[name].filter((n) => roots[name].contains(n)).length;
  const joined = split[name].length === 0 ? "" : `; split off: ${split[name].length}, in the page still: ${left}`;
  console.log(`${name}: ${stayed.length} of ${server.length} server nodes kept, ${order}; new: ${news}${joined}`);
}
const turn = () => new Promise((r) => setTimeout(r, 0));
async function click(selector) {
  document.querySelector(selector).click();
  await turn();
}

const { default: init } = await import("./dist/app.js");
await init();
for (const name in roots) {
  show(name);
  kept(name);
}
console.log(`counter: the input's value is ${roots.counter.querySelector("input").value}`);
const same = roots.counter.querySelector("#note") === note;
console.log(`counter: the textarea is the server's: ${same}; its value is "${note.value}"`);
for (const name of ["long", "longer"]) {
  console.log(`${name}: its text nodes' lengths, parsed: ${parsed[name]}; hydrated: ${lengths(name)}`);
}

// The counter's adopted nodes follow its signal.
await click("#inc");
show("counter");
kept("counter");
console.log(`counter: the input's value is ${roots.counter.querySelector("input").value}`);

// An adopted row's handler; then the regions build again between the
// server's markers, which stay, but for the rows' own, which go with them.
const markers = before.lists.filter((n) => n.nodeName === "#comment" && n.parentNode.localName !== "li");
const rows = [...roots.lists.querySelectorAll("#rows li")].slice(1, 3);
rows[1].click();
await turn();
for (const step of ["#toggle", "#toggle", "#more"]) {
  await click(step);
  show("lists");
}
console.log(`lists: the server's ${markers.length} outer markers are in the page: ${markers.every((m) => roots.lists.contains(m))}`);
console.log(`lists: a row the server wrote is in the page: ${rows.some((r) => roots.lists.contains(r))}`);
roots.lists.querySelectorAll("#rows li")[3].click();
await turn();

// The dynamic text writes to its own node, between the separators.
await click("#word");
show("texts");
kept("texts");

// The guests' list builds again with one more.
await click("#guest");
show("data");
console.log(`console errors: ${win.errors.length}`);
