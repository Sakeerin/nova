# Nova for VS Code

Colouring for `.nova` files, and from `nova lsp`: diagnostics, completion,
formatting, hover, go to definition, find references and rename.

The extension runs the `nova` you have installed: the `nova.server.path`
setting, or else `nova` from your PATH. Install it with

    cargo install --locked --git https://github.com/Sakeerin/nova nova-cli

**Nova: Restart Language Server** restarts it, for example after installing
a newer `nova`.
