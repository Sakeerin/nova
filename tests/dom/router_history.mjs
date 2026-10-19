// Drive `router_history.nova` a turn apart, printing the location's path
// and query and the routed part of the page after each step, and how many
// history entries a second click on the same link adds. A window listener
// added after the router's says whether each click's default was prevented.
import { install } from "./fake_dom.mjs";
install();
const app = document.createElement("div");
app.setAttribute("id", "app");
document.body.appendChild(app);
const { default: init } = await import("./dist/app.js");
await init();
window.addEventListener("click", (e) => console.log(`click: prevented=${e.defaultPrevented}`));
const turn = () => new Promise((r) => setTimeout(r, 0));
const page = document.getElementById("page");
const show = (what) => console.log(`${what}: ${location.pathname}${location.search} ${page.innerHTML}`);
const click = async (id, modifiers) => {
  document.getElementById(id).click(modifiers);
  await turn();
};
console.log(`hrefs: ${document.querySelectorAll("a").map((a) => a.getAttribute("href")).join(" ")}`);
show("at /");
await click("post");
show("link clicked");
const once = history.length;
await click("post");
show("clicked again");
console.log(`the same link again: ${history.length - once} more entries`);
await click("search");
show("navigated");
history.back();
await turn();
show("back");
await click("elsewhere");
show("unmatched");
await click("cafe");
show("encoded");
await click("params");
const encoded = history.length;
await click("same");
console.log(`the same place written decoded: ${history.length - encoded} more entries`);
// A delimiter written percent-encoded is not the delimiter: each of these
// goes elsewhere, and pushes an entry.
for (const [from, to] of [
  ["slashed", "encoded-slash"],
  ["amp", "encoded-amp"],
]) {
  await click(from);
  show(from);
  const before = history.length;
  await click(to);
  show(to);
  console.log(`${to} after ${from}: ${history.length - before} more entries`);
}
await click("frag");
show(`a fragment, ${location.hash}`);
await click("frag-query");
show(`a fragment after a query, ${location.hash}`);
for (const modifiers of [{ ctrlKey: true }, { metaKey: true }, { shiftKey: true }, { altKey: true }, { button: 1 }]) {
  console.log(`modifiers: ${JSON.stringify(modifiers)}`);
  await click("post", modifiers);
}
console.log("a link with target");
await click("blank");
console.log("a link with download");
await click("download");
console.log("a plain <a>");
await click("plain");
show("left to the browser");
await click("inner");
show("inner clicked");
console.log(`history: ${history.length} entries`);
await click("unmount");
show("unmounted");
await click("post");
show("after unmount");
await click("search");
console.log("after navigate");
