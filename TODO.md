# Hapsburg Roadmap

How this project actually got here, phase by phase, and what's left.
[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) is the technical design
behind each phase; this file is the history and the punch list.

## Phase 0 — The pitch ✅

Hapsburg started as a bit in a chat conversation, not a spec: a language
where mandatory multiple inheritance and the diamond problem are the
whole premise, sketched out in prose with fictional Advent of Code 2017
Day 1–3 solutions and a performance comparison chart explicitly labeled
"illustrative, not benchmarked — there's no real Hapsburg compiler to
profile." No code existed yet.

## Phase 1 — A real compiler ✅

- [x] Lexer, recursive-descent parser, AST (`compiler/src/{lexer,parser,ast}.rs`)
- [x] Resolver: genuine [C3 linearization](https://en.wikipedia.org/wiki/C3_linearization),
      not a stand-in — `founder`/parent-count rules, the "no genetic
      diversity" refusal, a real thrown `InbreedingError` on inconsistent
      ancestry (`compiler/src/resolve.rs`)
- [x] Codegen: transpile to C via whole-program per-class method
      monomorphization — correct override semantics with zero vtables
      (`compiler/src/codegen.rs`, `docs/ARCHITECTURE.md` §5)
- [x] `ferdinand file.hb -o out` produces a real native binary, shelling
      out to `cc -O2`
- [x] All three AoC 2017 Days 1–3 from the original pitch, implemented
      for real and verified against known answers
      (`examples/aoc2017/`)
- [x] Three negative examples proving the type system's actual behavior
      (`examples/negative/`: `diamond_conflict.hb`, `abstract_birth.hb`,
      `no_genetic_diversity.hb`)

  → This is the phase that answered the feasibility question the
  original chat's chart couldn't: C3 linearization and whole-program
  monomorphization are both real, implementable, and the performance
  story survives contact with an actual compiler. See
  `docs/ARCHITECTURE.md` §3–§7.

## Phase 2 — Tooling ✅

- [x] Nix flake: `nix develop` (dev shell), `nix build`/`nix run` (both
      binaries, wrapped with `cc` on `PATH` since `ferdinand` needs it at
      *run* time — `flake.nix`)
- [x] Neovim syntax highlighting: traditional Vim syntax rules, no
      tree-sitter grammar (`editors/nvim/`)
- [x] `hapsburg-lsp`, diagnostics only at first: real parser/resolver
      errors and warnings published as LSP diagnostics (`lsp/`)

## Phase 3 — LSP: hover, symbols, definition, completion ✅

- [x] Restructured `compiler` into lib+bin so the LSP reuses the real
      compiler instead of reimplementing it (`compiler/src/lib.rs`)
- [x] Hover backed by real resolved data: a dynasty's actual C3
      pedigree and trait/method table, a local variable's actual
      inferred type (`lsp/src/analysis.rs`)
- [x] Document symbols, go-to-definition, completion
- [x] Graceful degradation: one class's `InbreedingError` doesn't blank
      out hover for well-formed classes elsewhere in the file
      (`docs/ARCHITECTURE.md` §8.1)

## Phase 4 — Syntax redesign: inheritance everywhere ✅

The language originally stopped the bit at the class level — a local
variable was still a plain `let x: Type = value`. Redesigned so
`descends` is the *only* way to relate a name to a type anywhere in the
grammar, not just for classes:

- [x] `let` → `heir`; `trait`/parameter/local type annotations all use
      `descends`, not `:` (`SPEC.md` §3)
- [x] `Integer::parse(s)` → `value.marry(Type)`, a real bidirectional
      cast operator with a closed conversion table (`SPEC.md` §3.2)
- [x] `Habsburg::Correspondence::receive_line()`/`receive_all()` — real
      stdin reading
- [x] All three AoC examples switched from hardcoded test strings to
      actually reading piped stdin input

## Phase 5 — Tests, CI, formal documentation ✅

- [x] Unit tests across the lexer, parser, resolver, and codegen —
      including a test that inspects generated C to confirm inherited
      methods dispatch through the leaf class's own overrides
- [x] `compiler/tests/golden.rs`: real end-to-end tests (compile, `cc`,
      run, check exact output) against every example
- [x] `lsp/src/analysis.rs` unit tests, including the graceful-
      degradation regression test
- [x] `.github/workflows/ci.yml`: fmt, clippy, build, test on every push/PR
- [x] `docs/SPEC.md` (language specification) and `docs/ARCHITECTURE.md`
      (design rationale) written

  → Writing `ARCHITECTURE.md` honestly surfaced two real gaps by testing
  claims instead of assuming them — see "Known gaps," below.

## Phase 6 — Documentation restyle ✅

- [x] `docs/ARCHITECTURE.md` restructured into numbered sections
- [x] `docs/CHEATSHEET.md` — one-page quick reference
- [x] `TODO.md` (this file)
- [x] `README.md` — `## Status` section, cross-links updated

## Known gaps / next up

Full explanation of each lives in
[`docs/ARCHITECTURE.md` §10](docs/ARCHITECTURE.md#10-known-gaps); this is
just the punch list.

- [ ] `self` inside a `.map(|x| ...)` lambda leaks a raw C compiler error
      instead of a clean Hapsburg diagnostic (pinned by
      `examples/negative/self_in_lambda.hb`, currently expected to fail
      this way — fixing it means updating that test, not just the compiler)
- [ ] `heir name descends Type = expr` doesn't cross-check `Type` against
      `expr`'s inferred type (`birth(...)`'s `field:` arguments already
      do this — same check needs to land in `Stmt::Heir`'s codegen)
- [ ] No `super` — mechanical to add once `FnCtx` threads through which
      ancestor to resolve against
- [ ] No column spans, only line numbers, and not for every codegen-stage
      error — the single change that would also unlock precise hover
      (a real position→AST-node lookup instead of word-under-cursor) and
      LSP rename/references
- [ ] `hapsburg-lsp` has no automated wire-protocol test — `analyze()`'s
      pure logic is unit-tested, but nothing currently re-checks that
      the JSON-RPC layer serializes it correctly on every change

## Stretch goals

Real features from the original pitch, deliberately out of scope for v1
because none of the three AoC examples needed them — not oversights, see
`docs/ARCHITECTURE.md` for why each was cut:

- Runtime polymorphism (a `List` of *some* subtype, dispatched per-element
  at runtime) — the one thing that would make whole-program
  monomorphization stop being free, so it needs real design work, not
  just implementation time
- Real generics (`List<T>` for arbitrary `T`, not three hand-specialized
  container types)
- `dominant`/`recessive` trait conflict markers (traits that only
  "express" when inherited through both parent lines)
- `abdicate`/`Regency` deprecation shims
- A refcounted memory model with logged `cause_of_death` per object
  (today: everything is `malloc`, process lifetime, nothing freed —
  fine for short CLI programs, not for a long-running one)
- Regional stdlib renaming (`Habsburg::Netherlands` etc. as the
  collection types, per the original pitch's flavor text)
