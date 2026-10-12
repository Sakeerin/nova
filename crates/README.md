# Part of the Nova toolchain

This crate is one of the libraries that make up `nova`, the toolchain of
the [Nova programming language](https://github.com/Sakeerin/nova): its
compiler, formatter, language server, package manager and documentation
generator.

Its API is internal to `nova`. It is published only so that `nova` can be
installed from crates.io, and it changes with every release, without
notice. Every crate of a release depends on the others at exactly that
release's version.

To install `nova`:

```sh
cargo install nova-cli --locked
```

The [repository](https://github.com/Sakeerin/nova) has the language's
documentation, the ready-built releases and the install scripts. The
crates are licensed under either of the MIT licence or the Apache License,
Version 2.0, at your option.
