# Hapsburg

A compiled programming language where the only way to create anything is to
inherit from something. Multiple inheritance is mandatory. The diamond
problem is not solved, it's the primary debugging experience. This repo is
a feasibility exploration: does that premise survive contact with a real
compiler, and can it be made to run fast?

The short answer: yes to both, with real caveats documented below.

## What's actually here

`ferdinand`, a compiler written in Rust that transpiles `.hb` source to C
and shells out to `cc -O2`. It is a real, working, from-scratch compiler:
lexer, recursive-descent parser, a resolver that performs genuine
[C3 linearization](https://en.wikipedia.org/wiki/C3_linearization) over the
inheritance graph, and a C code generator. It is not a toy interpreter —
`ferdinand file.hb -o out` produces a native binary.

```
cargo build --release             # builds the whole workspace from the repo root
./target/release/ferdinand examples/aoc2017/day1.hb -o /tmp/day1
echo 1122 | /tmp/day1
```

Or, with Nix: `nix develop` for a shell with the right Rust toolchain and
`cc` on `PATH`, or `nix run . -- examples/aoc2017/day1.hb -o /tmp/day1` to
run it without installing anything. See **Development** below.

All three of the AoC 2017 examples originally sketched out in prose (Days
1–3) are implemented for real in `examples/aoc2017/` and produce correct
answers. `examples/negative/` contains three programs that are *supposed*
to fail to compile, demonstrating the type system's actual behavior rather
than just asserting it in a pitch.

## The core idea, made real

Every dynasty (class) is declared with `descends` (`extends`, but it wants
multiple parents), fields are `traits`, and instances are `birth()`'d
rather than `new`'d. A class with no parents must say so explicitly
(`founder`); everything else must have at least one ancestor.

```
dynasty AdventOfCode::Y2017::Day1 descends Puzzle::Solution {
    override offset() -> Integer {
        return 1
    }
    override solve() -> Integer {
        // ...
    }
}

dynasty AdventOfCode::Y2017::Day1::PartTwo descends AdventOfCode::Y2017::Day1 {
    override offset() -> Integer {
        return self.input.length / 2
    }
}
```

`solve()` is never redeclared in `PartTwo` — it's inherited untouched, and
overriding the one trait it depends on (`offset`) is enough to get Part 2's
behavior. That's `examples/aoc2017/day1.hb`, and it prints the correct `3`
and `6`.

### C3 flattening, not runtime dispatch

The performance question from the original design conversation was: do you
resolve multiple-inheritance conflicts at every method call (cheap to
build, expensive to run), or once at compile time (expensive to build,
cheap to run)? This compiler always takes the second path, because the
whole program is compiled as one closed-world unit — there's no separate
compilation, so nothing is unknown at codegen time.

Concretely: for every class that's actually `birth()`'d, `ferdinand`
computes its C3 linearization, resolves every trait and method to a single
winning definition, and then generates **one dedicated C function per
resolved method, monomorphized for that exact class** — even for methods
the class never wrote itself. When `PartTwo::solve()`'s body (inherited
from `Day1`) calls `self.offset()`, the generated code is a direct call to
`hb__AdventOfCode__Y2017__Day1__PartTwo__offset`, not `Day1`'s version —
correct override semantics with zero vtables, zero function pointers, and
zero runtime dispatch cost anywhere in the program. You can see this for
yourself:

```
ferdinand examples/aoc2017/day1.hb -o /tmp/day1 --emit-c
```

This is a real instance of the tradeoff the original chat only drew as an
illustrative chart: whole-program devirtualization is cheap to get when
there's no separate compilation to preserve, and the price is paid at
compile time (more generated code — one copy of every inherited method per
concrete class) rather than at runtime. `--show-pedigree` prints the
linearization ferdinand computed for each class, e.g.:

```
ferdinand: pedigree of 'AdventOfCode::Y2017::Day1::PartTwo': AdventOfCode::Y2017::Day1::PartTwo -> AdventOfCode::Y2017::Day1 -> Puzzle::Solution
```

### InbreedingError is a real, thrown compile error

The joke was always that the diamond problem should be the primary
debugging challenge, not a footnote. It is: `resolve.rs` implements
standard C3 merge, and when it fails (the classic inconsistent-MRO case —
two ancestors demanded in incompatible orders), `ferdinand` reports a real
`InbreedingError` naming the conflicting lines of descent, before any code
is generated:

```
$ ferdinand examples/negative/diamond_conflict.hb -o /tmp/x
InbreedingError: line 24: cannot reconcile the ancestry of 'E' — conflicting
lines of succession among A, B. Pedigree so far: []
```

`examples/negative/abstract_birth.hb` shows the other compile-time check
that actually matters: you cannot `birth()` a dynasty that still has an
unimplemented `abstract` method anywhere in its resolved method table.
`examples/negative/no_genetic_diversity.hb` demonstrates the pitch's
"fully connected inheritance graph" refusal — worth reading the comment in
that file, because it's less trivial than it sounds: a *plain* single-line
chain is always a total order (every class comparable to every other), so
naively checking "is the graph fully connected" would reject nearly all
ordinary single-inheritance programs. The check only fires when the
program actually uses multiple inheritance somewhere and *still* ends up
fully connected — i.e. the extra parent didn't bring in a new bloodline.

### Inheritance, all the way down

The first version of this language stopped the bit at the class level: a
`dynasty` descended from its parents, but a local variable was just a
mundane `let x: Type = value` — the one place in the whole file that
looked like it belonged to any other language. That's gone. There is now
exactly one relationship in Hapsburg for relating a name to a type,
`descends`, and it's the same word whether the name is a whole dynasty, a
field, a parameter, or a single local:

```
heir digits descends List<Integer> = self.input.chars().map(marry(Integer))
heir n descends Integer = digits.length
```

`heir` replaced `let` (an heir, descending from a type, given a value —
not a stretch, an heir is definitionally something that inherits).
Casting got the same treatment: `Integer::parse(s)` is gone, replaced by
`value.marry(Type)` — converting a value's type is now phrased as
marrying it into a new bloodline, which is either the most or least
defensible pun in the codebase depending on how charitable you're
feeling about `Bella gerant alii, tu felix Austria, nube`. It's
bidirectional (`n.marry(String)` round-trips), same-type marriage is a
no-op, and an unsupported pair is refused at compile time with exactly
the tone you'd expect: `no legitimate marriage between Bool and Integer`.

Reading input followed the same instinct: `Habsburg::Correspondence::
receive_line()` / `receive_all()` read stdin, framed as the crown
receiving petitions from outside the realm. The AoC examples actually
read real puzzle input now (`echo 1024 | ferdinand-compiled-day3`)
instead of hardcoding a test string in `main` — a straightforwardly
better demo of "the language can read input" than the pre-redesign
version had, independent of the theming.

## Language reference (v1)

| Syntax | Meaning |
|---|---|
| `dynasty X descends A, B` | class declaration; multiple parents allowed and normal |
| `founder` | marks a class with no parents (required if it has none) |
| `trait name descends Type [= default]` | a field |
| `override name(param descends Type, ...) -> Type { ... }` | method; body can be `abstract` |
| `birth(Class, field: value, ...)` | construct an instance |
| `heir name [descends Type] = expr` | local variable binding (type inferred if omitted) |
| `value.marry(Type)` | cast `value` into `Type`; also usable bare as `marry(Type)` inside `.map(...)` |
| `succession over EXPR as NAME { }` | foreach; also accepts `Habsburg::Range::infinite()`, `Habsburg::Range::up_to(n)`, and `LIST.indices` |
| `claim COND { } contested { }` | if / else |
| `assassinate(ExceptionName, reason: "...")` | terminate with a runtime error |
| `Habsburg::Correspondence::receive_line()` / `receive_all()` | read one line / everything from stdin, both `-> String` |
| `self` | the current instance (always statically typed — see below) |

Types: `Integer`, `String`, `Bool`, `List<Integer>`, `List<String>`,
`List<List<Integer>>`, and dynasty types. `Habsburg::Accumulator` (a
`seed:`-birthed running total with `.absorb()`/`.value`) is a compiler
intrinsic, not a user-space dynasty, in this version.

## What's real vs. what's future work

The original pitch had more surface area than a feasibility spike needed
to cover. Implemented for real:

- Mandatory multiple inheritance, `founder`, C3 linearization, `InbreedingError`
- Compile-time flattening / per-class method monomorphization (real devirtualization)
- The "no genetic diversity" refusal
- A working type checker for the language's small type surface
- A real C runtime (lists, strings, `.marry()` casts, `Habsburg::Accumulator`, stdin via `Habsburg::Correspondence`)

Deliberately deferred, because they add real complexity for no payoff on
the three AoC examples that anchored this spike:

- **The refcounted / `cause_of_death` memory model.** v1 never frees
  anything — every `birth()` is a `malloc` with process lifetime. Fine for
  a short-lived CLI puzzle solver, not fine for a long-running program.
  `Hemophilia` (leaked cycles) and `assassinate()`-as-manual-free are
  unimplemented.
- **`dominant`/`recessive` trait conflict semantics.** Currently every
  conflict resolves the same way method resolution does: most-derived
  definition wins, full stop. The pitch's idea of traits that only
  "express" when inherited through *both* parent lines is a genuinely
  different (and more work to implement correctly) mechanic.
- **`abdicate` / `Regency` deprecation shims, `super` calls.**
- **Real generics.** `List<T>` is three hand-specialized container types
  (`List<Integer>`, `List<String>`, `List<List<Integer>>`), not a real
  generic — adequate for the examples, not for a general-purpose language.
- **Runtime polymorphism.** Every expression's type is statically known to
  be one concrete class; there's no way to hold a supertype-typed
  reference to different subtype instances and dispatch dynamically. This
  is *why* full devirtualization is free here — the moment you need
  heterogeneous collections of a common ancestor, you need either a real
  vtable fallback or whole-program specialization at every call site, and
  that's the "naive vs. flattened" tradeoff the original chat's charts
  gestured at without ever building either side.
- **The regional stdlib naming (`Habsburg::Netherlands`, etc.), `Habsburg::Spain`/`Austria` collections.**

## What we actually learned about performance

The original conversation's performance charts were explicitly invented —
"illustrative, not benchmarked... there's no real Hapsburg compiler to
profile." There is now, so here's one real measurement instead of a
plausible-looking radar chart.

Day 1's algorithm run over a 3,000,000-digit synthetic input, compiled
with `cc -O2`, compared to the equivalent straight-line Python loop:

| | user CPU time |
|---|---|
| `ferdinand`-compiled binary | ~0.16–0.24s |
| CPython 3 (equivalent loop) | ~0.49s |

(Wall-clock time on this measurement was noisy — dominated by sandbox
syscall overhead unrelated to the program — so user CPU time is the
comparable number here, not wall time.)

The compiled binary wins, but not by as much as "AOT-compiled to C" ought
to win, and the reason is instructive: `self.input.chars().map(...)`
allocates one heap string *per character* (3 million small `malloc`s) in
v1's `hb_string_chars`, because the runtime represents `List<String>` as
an array of individually-heap-allocated C strings rather than, say, a
packed byte buffer with borrowed slices. The C3-flattening /
devirtualization story is real and is exactly as cheap as claimed — every
method call in the generated code is a direct call, confirmed by reading
the emitted C. But it doesn't matter if the standard library sitting on
top of it allocates carelessly. That's a more honest and more useful
finding than the original chart: **the dispatch mechanism was never the
expensive part; the naive collection representation is.** A real v2 would
fix `hb_string_chars` before touching dispatch again.

## Development

The repo is a Cargo workspace (`ferdinand` the compiler, `hapsburg-lsp` the
language server) with a Nix flake wrapping it:

```
nix develop                 # rustc, cargo, clippy, rustfmt, rust-analyzer, and cc on PATH
cargo build --workspace     # or just use plain cargo once you're in the shell
nix build                   # produces ./result/bin/{ferdinand,hapsburg-lsp}
nix run . -- file.hb -o out
```

`ferdinand` shells out to `cc` at *runtime* (it compiles the C it
generates), which is why the flake wraps both binaries with `cc` on
`PATH` rather than only making it available at build time.

Without Nix, any recent stable Rust toolchain (`cargo build --release`
from the repo root) and a C compiler on `PATH` are all you need.

## Editor support

- **Neovim syntax highlighting**: `editors/nvim/` — traditional Vim
  syntax rules (no tree-sitter grammar to compile) for filetype
  detection and highlighting of `.hb` files. See `editors/nvim/README.md`
  for install instructions.
- **LSP**: `lsp/` — `hapsburg-lsp`, a language server that runs the real
  compiler's parser, resolver, and a dry-run codegen in-memory. Live
  diagnostics (`InbreedingError`, the founder/diamond/genetic-diversity
  checks, "single line of descent" warnings); hover with real resolved
  data (a dynasty's actual C3 pedigree and trait/method list, a local
  variable's actual inferred type — not guesses); document symbols;
  go-to-definition; and completion. See `lsp/README.md` for exactly what
  it does and doesn't cover, and the `nvim-lspconfig` snippet to wire it
  up.

## Repo layout

```
Cargo.toml           workspace root (members: compiler, lsp)
compiler/            the ferdinand compiler (Rust)
  src/lexer.rs        tokenizer
  src/parser.rs       recursive-descent parser -> AST
  src/ast.rs          AST types
  src/resolve.rs      C3 linearization, founder/diamond/genetic-diversity checks
  src/codegen.rs      per-class method monomorphization -> C
  src/lib.rs          exposes the above as a library (used by both main.rs and lsp/)
  src/main.rs         CLI: parse -> resolve -> codegen -> cc
lsp/                 hapsburg-lsp, a diagnostics-only language server
runtime/             the C runtime ferdinand-generated code links against
examples/aoc2017/    Advent of Code 2017 Days 1-3, real working programs
examples/negative/   programs that are supposed to fail to compile, and do
editors/nvim/        Neovim filetype detection + syntax highlighting
flake.nix            Nix dev shell + package (wraps both binaries with `cc` on PATH)
```
