# Neovim support for Hapsburg

Filetype detection and syntax highlighting for `.hb` files (traditional
Vim syntax rules, not tree-sitter — works in any Vim or Neovim without a
grammar compile step).

## Install

With [lazy.nvim](https://github.com/folke/lazy.nvim), pointing at a local
checkout:

```lua
{ dir = "/path/to/Hapsburg/editors/nvim", name = "hapsburg.nvim" }
```

Or by hand, add the directory to your runtimepath in `init.lua`:

```lua
vim.opt.rtp:append("/path/to/Hapsburg/editors/nvim")
```

If you use the flake's dev shell (`nix develop`), the repo (and therefore
this directory) is already on disk at a known path, so either method just
needs the path filled in.

## What you get

- `*.hb` files are recognized as filetype `hapsburg`
- Syntax highlighting for keywords (`dynasty`, `descends`, `founder`,
  `trait`, `override`, `abstract`, `birth`, `succession`/`over`/`as`,
  `claim`/`contested`, `heir`, `return`, `self`), booleans, `List`/
  `Integer`/`String`/`Bool`/`Void` types, `marry`, `::`-qualified dynasty paths,
  strings, numbers, and `//` comments
- `commentstring` set to `// %s` (so `gcc`/commenting plugins work)

## LSP

See `../../lsp/README.md` for `hapsburg-lsp`, and the snippet there for
wiring it into `nvim-lspconfig`.
