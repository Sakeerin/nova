// nova dev's reload script (spec
// docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md
// §4.5), which the server adds to every page it serves and never writes
// into a file. Its tag carries `data-build`, the id of the build current
// when the server served the page. It listens to /__nova/events: a `hello`
// whose id is not the page's own reloads the page, so a page served before
// a build it missed reloads, and so does every page after a restart;
// `reload` reloads it; `error` shows the build's errors over it, until a
// good build reloads it. A page served after a failed build carries that
// build's id, so its `hello` matches, and the `error` after it shows.
(() => {
  // Read once, while the script runs: `currentScript` is null afterwards.
  const served = document.currentScript?.dataset.build ?? null;
  const events = new EventSource("/__nova/events");

  // A tag without an id never reloads on `hello`, rather than on every one.
  events.addEventListener("hello", (e) => {
    if (served !== null && e.data !== served) location.reload();
  });

  events.addEventListener("reload", () => location.reload());

  // The stream's own failures arrive as `error` too, as plain events with
  // no data; EventSource reconnects by itself after them.
  events.addEventListener("error", (e) => {
    if (typeof e.data === "string") show(e.data);
  });

  // The overlay: a fixed layer over the whole page, its text set as
  // `textContent`, so that nothing in an error is read as HTML.
  function show(text) {
    let overlay = document.getElementById("nova-dev-overlay");
    if (overlay === null) {
      overlay = document.createElement("div");
      overlay.id = "nova-dev-overlay";
      overlay.style.cssText =
        "position:fixed;inset:0;z-index:2147483647;overflow:auto;box-sizing:border-box;" +
        "margin:0;padding:24px;background:rgba(24,24,27,0.96);color:#f4f4f5;" +
        "font:14px/1.5 ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;";
      const title = document.createElement("h1");
      title.textContent = "nova dev: the build failed";
      title.style.cssText = "margin:0 0 16px;font-size:18px;color:#fca5a5;";
      const pre = document.createElement("pre");
      pre.style.cssText = "margin:0;white-space:pre-wrap;";
      overlay.append(title, pre);
      (document.body ?? document.documentElement).append(overlay);
    }
    overlay.querySelector("pre").textContent = text;
  }
})();
