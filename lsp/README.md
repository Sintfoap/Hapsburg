# hapsburg-lsp

A language server for Hapsburg. It runs the real compiler pipeline
(`ferdinand`'s parser, resolver, and a dry-run of codegen that never
touches disk or invokes `cc`) in-memory on every open/change/save, and
serves diagnostics, hover, document symbols, go-to-definition, and
completion off of what that pipeline actually resolved — none of it is a
separate reimplementation of the compiler's semantics.

## What it does

**Diagnostics.** Parse errors, `InbreedingError`, the `founder`/parent-
count rules, the "no genetic diversity" refusal, and "cannot birth an
abstract dynasty" show up as real-time error diagnostics — the actual
compiler messages. The "single line of descent" notices show up as
warnings. Unlike the CLI (which stops at the first error anywhere),
codegen-stage diagnostics are collected per-class rather than truncated
after the first class that fails, so editing one broken method doesn't
hide errors elsewhere in the file. Full-document sync
(`TextDocumentSyncKind::FULL`) — simplest correct thing for a compiler
that always re-checks the whole file anyway.

**Hover.** Backed by real data, not guesses:
- A dynasty name shows its founder/parent status, its actual C3
  linearization (pedigree), and its fully resolved trait and method
  lists (with `(abstract)` / `— inherited from X` annotations pulled
  straight from the resolver).
- A local variable or parameter shows its real inferred type (e.g.
  `heir digits descends List<Integer>`), from the same type inference
  codegen uses to emit C — not a syntactic guess.
- Keywords and builtins (`dynasty`, `succession`, `Habsburg::Accumulator`,
  etc.) get static reference docs.

Resolution degrades gracefully: a class with a genuine `InbreedingError`
elsewhere in the file doesn't block hover on the classes that *are*
well-formed (verified against `examples/negative/diamond_conflict.hb` —
`A` and `B` still get full hover info even though `E` fails to
linearize).

**Document symbols**, nesting each dynasty's traits and methods —
useful as a file outline / breadcrumb.

**Go-to-definition** on a dynasty name, jumping to its `dynasty` line.

**Completion**: keywords, builtins, and every dynasty name declared in
the current document (unfiltered — no context-awareness about what's
valid at the cursor).

## What it doesn't do (yet)

- Hover/completion/definition resolve the word under the cursor by
  scanning the source line's text, not by mapping a position to an AST
  node — because the AST doesn't carry column spans (see below). In
  practice this means they key off identifier *names*, which is exactly
  right for dynasty/trait/method/keyword lookups (those are global by
  name) and works for local variables via "nearest declaration of this
  name at or before this line" — a real but line-granularity, not
  scope-precise, approximation. Two same-named locals in sibling blocks
  on the same file could theoretically be ambiguous; not an issue in
  practice for programs this size.
- **Line numbers, not columns, and not for every diagnostic.** ferdinand
  tracks source lines for most parser/resolver errors but not yet for
  every codegen-stage type error (see the main README's "what's real vs.
  future work" — the same honest gap applies here). Diagnostics for
  those fall back to line 1 rather than pointing at the wrong place.
  Threading real spans through the whole AST is the natural fix and
  would upgrade the compiler's own error messages and this server at
  once.
- No rename, references, or code actions.

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
