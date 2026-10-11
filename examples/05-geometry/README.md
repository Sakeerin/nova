# 05-geometry

Measures a few shapes with a library beside it, and documents both with
`nova doc`.

## What this demonstrates

- **A path dependency:** `shapes = { path = "shapes" }` in `nova.toml`,
  and `import shapes` in `src/main.nova`.
- **A library of two modules:** `shapes/src/lib.nova` is its API, and it
  uses its own `units.nova`.
- **Records, a sum type, a trait implemented for both, functions and a
  constant.**
- **Docs:** `///` on every item and member and `//!` on every module,
  which `nova doc` renders.

## Run it

```bash
cd examples/05-geometry
nova run
nova doc --open
```

## Expected output

```
area 3.141592653589793, perimeter 6.283185307179586
area 6, perimeter 10
total area 9.141592653589793
```

`nova doc` prints `target/doc/geometry/index.html`, the program's page,
which links to the library's pages and to std's.

## Notes

- A dependent can name only the items of a library's `lib.nova`, so the
  program never names `Length`, though it reads a `Length`'s `value`
  (`docs/adr/0030-package-modules.md`).
- A `Float` with no fraction prints without one: `6`, not `6.0`.
- Phase 3.5a's gate documents this example
  (`docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`
  §13, §17).
