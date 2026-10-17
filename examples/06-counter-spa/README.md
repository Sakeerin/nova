# 06-counter-spa

The smallest WASM frontend: a counter written as a component.

## What this demonstrates

- **`component` and `view { … }`** (RFC 0001): elements with attributes
  by name and children in a block, and `text(…)`.
- **Reactivity:** `text("Count: ${count.value()}")` reads a signal, so it
  is read again, alone, each time a button sets it.
- **`||`**, a closure with no parameters, for each button's handler.
- **The bundler:** the project's own `index.html` is copied into `dist/`,
  beside `app.js`, `app.wasm` and `manifest.json`.

## Run it

```bash
cd examples/06-counter-spa
nova build --target wasm
cd dist && python3 -m http.server 8000
```

Then open `http://localhost:8000/` in a browser. Any static server works,
as long as it serves `app.wasm` as `application/wasm`.

## Expected output

The page shows `Count: 0` and two buttons. `+` adds one and `-` takes one
away: three clicks of `+` and one of `-` leave `Count: 2`.

## Notes

- It differs from `nova-spec/60-EXAMPLES.md` §6 in three places, each the
  API 4.3 built: `import ui` and `import ui::html::{button, div}`,
  `count.value()`, and `mount("#app", Counter)`.
- `nova build --target wasm --release` runs `wasm-opt -Oz` on `app.wasm`
  when `wasm-opt` is on `PATH`; the gate's size, under 50 KB of `app.wasm`
  and `app.js` gzipped, is measured without it.
- `cargo test` clicks it under a fake DOM, and CI's `browser` job in
  Chromium (`tests/browser/spa.mjs`); the 4.4 spec is
  `docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md`.
