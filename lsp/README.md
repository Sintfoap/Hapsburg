# hapsburg-lsp

A minimal language server for Hapsburg. It runs the real compiler
pipeline (`ferdinand`'s parser and resolver, plus a dry-run of codegen
that never touches disk or invokes `cc`) in-memory on every open/change/
save and publishes whatever it reports as LSP diagnostics.

## What it does

- Parse errors, `InbreedingError`, the `founder`/parent-count rules, the
  "no genetic diversity" refusal, and "cannot birth an abstract dynasty"
  all show up as real-time error diagnostics — not a reimplementation,
  the actual compiler messages.
- The "single line of descent" notices show up as warnings.
- Full-document sync (`TextDocumentSyncKind::FULL`): simplest correct
  thing for a compiler that always re-checks the whole file anyway.

## What it doesn't do (yet)

- **No completion, hover, or go-to-definition.** Diagnostics only.
- **Line numbers, not columns, and not for every error.** ferdinand
  tracks source lines for most parser/resolver errors but not yet for
  every codegen-stage type error (see the main README's "what's real vs.
  future work" — the same honest gap applies here). Diagnostics for
  those fall back to line 1 rather than pointing at the wrong place.
  Threading real spans through the whole AST is the natural fix and
  would upgrade both the compiler's own error messages and this server
  at once.
- Diagnostics stop at the first hard error per pass, same as the CLI.

## Running it

```
cargo run -p hapsburg-lsp
```

It speaks LSP over stdio, so in practice your editor launches it for
you.

## nvim-lspconfig

`hapsburg-lsp` isn't in lspconfig's built-in registry, so register it
yourself (Neovim 0.11+ `vim.lsp.config` API):

```lua
vim.lsp.config('hapsburg_lsp', {
  cmd = { 'hapsburg-lsp' }, -- or an absolute path to the built binary
  filetypes = { 'hapsburg' },
  root_markers = { 'flake.nix', '.git' },
})
vim.lsp.enable('hapsburg_lsp')
```

Pair this with `../editors/nvim` for `*.hb` filetype detection and
syntax highlighting — the LSP needs `filetype=hapsburg` to attach.

If you're using the flake's `nix develop` shell, `cargo build -p
hapsburg-lsp --release` and point `cmd` at
`target/release/hapsburg-lsp`; `nix run .#hapsburg-lsp` also works
directly.
