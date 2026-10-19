// Drive `router_hash.nova` a turn apart, printing the hash and the routed
// part of the page after each step: the router shown, a link's click,
// `navigate` with a query
// (the Search button), the back button, a hash typed in the address bar
// that no route but `*` matches, `navigate` to `/posts/café` and to
// `/posts/a b`, which the hash holds percent-encoded, and a link to a
// percent-encoded parameter written without its `/`; then the router hidden,
// and a link's click left to the browser. The menu's `href`s are printed
// before the router shows, while it is shown, and once it is hidden. A
// window listener added once the router shows, after the router's, says
// whether each click's default was prevented.
import { FakeEvent, install } from "./fake_dom.mjs";
install();
const app = document.createElement("div");
app.setAttribute("id", "app");
document.body.appendChild(app);
const { default: init } = await import("./dist/app.js");
await init();
const turn = () => new Promise((r) => setTimeout(r, 0));
const page = document.getElementById("page");
const show = (what) => console.log(`${what}: ${location.hash || "(no hash)"} ${page.innerHTML}`);
const hrefs = (when) =>
  console.log(`hrefs ${when}: ${document.querySelectorAll("a").map((a) => a.getAttribute("href")).join(" ")}`);
hrefs("before the router shows");
document.getElementById("toggle").click();
window.addEventListener("click", (e) => console.log(`click: prevented=${e.defaultPrevented}`));
hrefs("with the router shown");
show("at /");
document.getElementById("post").click();
show("link clicked");
await turn();
show("a turn later");
document.getElementById("search").click();
await turn();
show("navigated");
// A browser fires `hashchange` after `popstate` when going back changes
// the hash; the fake DOM's `back` fires `popstate` alone.
history.back();
await turn();
window.dispatchEvent(new FakeEvent("hashchange"));
show("back");
location.hash = "#/no/such/page";
await turn();
show("typed");
document.getElementById("accent").click();
await turn();
show("café");
document.getElementById("space").click();
await turn();
show("a space");
document.getElementById("fragment").click();
await turn();
show("a second #");
document.getElementById("cafe").click();
await turn();
show("encoded");
document.getElementById("home").click();
await turn();
show("home");
console.log(`history: ${history.length} entries`);
document.getElementById("toggle").click();
hrefs("once the router is hidden");
document.getElementById("post").click();
await turn();
console.log(`a link with no router: ${location.hash}, ${page.innerHTML === "" ? "nothing routed" : page.innerHTML}`);
