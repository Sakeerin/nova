// Type into the field twice, a turn apart, printing the page and the
// fields' values after each; then click the button that sets two signals.
import { FakeEvent, install } from "./fake_dom.mjs";
install();
const app = document.createElement("div");
app.setAttribute("id", "app");
document.body.appendChild(app);
const { default: init } = await import("./dist/app.js");
await init();
const field = document.getElementById("name");
const show = () => {
  console.log(app.innerHTML);
  console.log(`name=${field.value} echo=${document.getElementById("echo").value}`);
};
show();
for (const typed of ["nova", ""]) {
  field.value = typed;
  field.dispatchEvent(new FakeEvent("input", { bubbles: true }));
  await new Promise((r) => setTimeout(r, 0));
  show();
}
document.getElementById("both").click();
await new Promise((r) => setTimeout(r, 0));
