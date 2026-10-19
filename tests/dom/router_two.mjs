// Drive `router_two.nova` a turn apart, printing the hash and both routers'
// parts of the page after each step: a link's click with both routers
// mounted, then the later router disposed, then a click on another link. A
// window listener added after the routers' says whether each click's default
// was prevented.
import { install } from "./fake_dom.mjs";
install();
const app = document.createElement("div");
app.setAttribute("id", "app");
document.body.appendChild(app);
const { default: init } = await import("./dist/app.js");
await init();
window.addEventListener("click", (e) => console.log(`click: prevented=${e.defaultPrevented}`));
const turn = () => new Promise((r) => setTimeout(r, 0));
const part = (id) => document.getElementById(id).innerHTML;
const show = (what) => console.log(`${what}: ${location.hash || "(no hash)"} first [${part("first")}] second [${part("second")}]`);
const click = async (id) => {
  document.getElementById(id).click();
  await turn();
};
console.log(`hrefs: ${document.querySelectorAll("a").map((a) => a.getAttribute("href")).join(" ")}`);
show("at /");
await click("post");
show("both mounted");
await click("dispose");
show("the later disposed");
await click("other");
show("left to the browser");
console.log(`history: ${history.length} entries`);
