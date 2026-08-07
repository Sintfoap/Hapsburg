# Hapsburg Architecture & Technical Design

This document is the *why* behind [`SPEC.md`](SPEC.md)'s *what*: the
architecture of `ferdinand` and `hapsburg-lsp`, the tradeoffs each major
decision made, and an honest accounting of what's still rough. See
[`../TODO.md`](../TODO.md) for how the project got here phase by phase; this
document is the technical design, not the history. If a claim here needs
grounding, it should point at either a source file or a test — anything
else is exactly the kind of unverified handwaving this project set out to
replace (§3).

## 1. Pipeline

```
 .hb source
      │
      ▼
 ┌────────┐   tokens   ┌────────┐    AST    ┌──────────┐  Resolved   ┌─────────┐   C source   ┌────┐
 │ Lexer  │ ─────────▶ │ Parser │ ─────────▶ │ Resolver │ ──────────▶ │ Codegen │ ───────────▶ │ cc │ ──▶ native binary
 └────────┘             └────────┘            └──────────┘             └─────────┘              └────┘
 lexer.rs               parser.rs              resolve.rs               codegen.rs
                         ast.rs                 (C3 linearization,       (per-class method
                                                  founder/diamond/         monomorphization)
                                                  genetic-diversity
                                                  checks)
```

No bytecode, no separate compilation: `ferdinand` reads the whole program,
resolves the entire inheritance graph, and emits one C translation unit
per invocation, then calls `cc -O2` on it. Every stage above is a plain
module in the `ferdinand` library crate (`compiler/src/lib.rs`), which
both the `ferdinand` binary and `hapsburg-lsp` depend on — see §7.

## 2. Crate layout

```
Hapsburg/
├── compiler/                 # the ferdinand compiler
│   ├── src/
│   │   ├── lexer.rs           # source text -> token stream
│   │   ├── ast.rs              # AST node definitions
│   │   ├── parser.rs            # token stream -> AST (recursive descent)
│   │   ├── resolve.rs            # C3 linearization + structural checks
│   │   ├── codegen.rs             # Resolved -> C (§4)
│   │   ├── lib.rs                  # exposes the above; used by main.rs and lsp/
│   │   └── main.rs                  # CLI: parse -> resolve -> codegen -> cc
│   └── tests/golden.rs        # end-to-end: compile+run every example
├── lsp/                      # hapsburg-lsp (§7)
│   └── src/
│       ├── analysis.rs        # runs the real pipeline in-memory, no cc
│       ├── docs_static.rs      # keyword/builtin hover text
│       └── main.rs              # tower-lsp wiring
├── runtime/                  # hapsburg_runtime.{h,c} -- what generated C links against
├── examples/
│   ├── aoc2017/               # real, working Advent of Code 2017 solutions
│   └── negative/              # programs that are supposed to fail, and do
├── editors/nvim/             # filetype detection + syntax highlighting
└── docs/
    ├── SPEC.md                # the language: grammar, types, semantics (source of truth)
    ├── ARCHITECTURE.md        # this file
    └── CHEATSHEET.md          # one-page quick reference
```

`resolve.rs` and `codegen.rs` never import each other's internals beyond
the `Resolved` struct `resolve.rs` exposes — `Codegen` only ever calls
`Resolver::resolve`/`linearize`, never reaches into its private
`lin_cache`. This is what makes `lsp/src/analysis.rs` able to reuse both
independently (call `resolve()` for hover data without ever running
`Codegen`, or run `Codegen` without needing its own copy of the C3 logic).

## 3. Where this started

Hapsburg began as a bit in a chat conversation: a language pitch where
"everything is inheritance" is taken to its logical, uncomfortable
extreme — mandatory multiple inheritance, the diamond problem as the
*primary* debugging experience rather than an edge case, `InbreedingError`
as a real exception name. That conversation also produced a performance
comparison chart, explicitly labeled "illustrative, not benchmarked —
there's no real Hapsburg compiler to profile."

This repository exists because someone asked what happens if you actually
build the compiler that chart pretended to have profiled. The honest
answer, worked out phase by phase (see [`TODO.md`](../TODO.md)): the core
joke has real, sound language-design content (C3 linearization is a real,
implementable algorithm; whole-program monomorphization really can give
you zero-vtable dispatch), and that content survives contact with a real
toolchain a lot better than the surrounding decoration does. What follows
documents the actual tradeoffs made getting there, not the ones a chart
could gesture at.

## 4. Why transpile to C, not a bytecode VM or LLVM

Three real options were on the table for "compiled and fast": a bytecode
VM, an LLVM backend, or transpiling to C and calling `cc`. C was chosen
because it gets the two things that actually mattered — a real native
binary, and an optimizing backend doing register allocation/inlining/loop
work the project has no interest in reimplementing — for a fraction of
the implementation cost of the other two. The cost is a hard runtime
dependency on a C compiler being on `PATH` (`ferdinand` shells out to
`cc` *at run time*, not just at its own build time — see `flake.nix`'s
comment on why the Nix package wraps both binaries with `cc` explicitly
on `PATH`, and `.github/workflows/ci.yml`'s explicit `cc --version`
check). An LLVM backend would drop that runtime dependency in exchange
for a much larger one at `ferdinand`'s own build time (`inkwell` or
similar) and a real jump in implementation complexity; a bytecode VM
trades both of those for reimplementing everything `-O2` already does
well. For a project whose actual output is three AoC solutions and a
point about C3 linearization, none of that complexity was buying
anything.

## 5. Whole-program monomorphization: the load-bearing design decision

The original chat's performance chart drew a line between "naive
Hapsburg" (resolve inheritance conflicts at every method call) and
"flattened Hapsburg" (resolve once, at compile time). `ferdinand` only
ever implements the second path, and the reason it can is specific and
worth stating precisely: **the whole program is compiled as one
closed-world unit.** There is no separate compilation, no dynamic
loading, and — this is the part that actually matters — no way in
Hapsburg's current type system to hold a variable whose static type is a
*supertype* while it points at a *subtype* instance at runtime (§5.1).
Every expression's concrete type is known at compile time, full stop.

Given that, `Codegen::gen_class` does something a bit unusual: for a
class `C`, it doesn't just generate code for the methods `C` itself
declares. It walks `C`'s *entire resolved method table* — including
methods `C` never wrote, inherited unchanged from some ancestor — and
generates a **fresh, independent C function for every single one**,
named `hb__C__methodname`. When that generated function's body calls
`self.foo()`, the call target is resolved against `C`'s own table, not
whichever ancestor originally wrote the method body. Concretely:
`AdventOfCode::Y2017::Day1::PartTwo` never redeclares `solve()` — it's
textually identical to `Day1::solve()` — but `ferdinand` still emits a
distinct `hb__AdventOfCode__Y2017__Day1__PartTwo__solve` function, and
the `self.offset()` call inside it compiles to a direct call to
`PartTwo`'s own `offset()` override, not `Day1`'s.
`codegen::tests::inherited_method_dispatches_through_the_leaf_classs_own_override`
asserts exactly this by inspecting the generated C text.

This is the whole trick: **correct override semantics (calling through
`self` respects overrides) with zero vtables, zero function pointers, and
zero runtime dispatch cost anywhere in the program**, because every call
site's target is a compile-time constant. `--emit-c` lets you read the
generated code and confirm it yourself; `--show-pedigree` prints the
linearization each class was generated from.

**The price**, paid honestly rather than hidden: code size. Every
concrete class gets its own copy of every method it inherits, not a
shared implementation reached through indirection. This is exactly the
"flattened Hapsburg pokes outward on compile time and binary size" shape
the original chat's chart predicted — except now it's a real,
inspectable property of real generated C instead of a guess.

### 5.1 What this forecloses

The reason whole-program monomorphization is *free* here is precisely
the reason it's not a general solution: Hapsburg has no way to write "a
`List` of things that are *some* subtype of `Puzzle::Solution`, resolved
per-element at runtime." Every `heir`/`trait`/parameter has one concrete
type, always. Adding real runtime polymorphism — a heterogeneous
collection dispatched dynamically — would need either a real vtable
fallback for that one case, or specializing every call site that touches
such a collection, which is a fundamentally different (and much larger)
project than this one. This is deliberately out of scope for v1; see §9.

## 6. The resolver

### 6.1 Why C3, not "just pick the first match"

The pitch's original text described diamond conflicts as "resolved via a
real, specified algorithm (call it C3 linearization with flavor text)."
C3 was picked because it's the *actual* algorithm real languages use for
this problem (Python's MRO), it has well-known failure semantics (the
"inconsistent precedence graph" case), and — practically — implementing
it correctly is what makes `InbreedingError` a real compiler behavior
instead of a string that only appears in documentation.
`resolve::c3_merge` is the standard algorithm, unmodified; the "flavor"
is entirely in the error message layered on top when it fails
(`resolve::Resolver::linearize`).

### 6.2 The genetic-diversity check's real subtlety

The pitch's "no genetic diversity, refusing to compile" line reads like
a throwaway joke, but implementing it honestly surfaced a real
mathematical fact worth recording: **a plain single-inheritance chain is
always a total order.** Every class in a straight line is comparable
(ancestor-or-descendant) to every other, which is exactly the "fully
connected" condition the check is looking for. A naive implementation of
the joke would therefore reject essentially every ordinary
single-inheritance program — which is obviously not the intent.

The fix (`Resolver::check_genetic_diversity`'s guard clause) is to only
run the check when the program uses multiple inheritance somewhere.
Working out a case that *still* trips the check under that guard is its
own small puzzle — `examples/negative/no_genetic_diversity.hb`'s comment
walks through it: `C descends B, A` where `B` already descends `A` isn't
a real second bloodline, and the parent order (`B` before `A`) has to
match `B`'s own ancestry or C3 itself refuses to merge it (the "married
into her own bloodline" framing in that file's comment isn't just
flavor — it's a precise description of the shape needed to trigger the
check).

### 6.3 Override resolution: simpler than the pitch, on purpose

The original pitch also described `dominant`/`recessive` trait markers —
a trait only "expressing" if inherited through *both* parent lines,
staying dormant otherwise. `ferdinand` implements the much simpler rule
that most single-dispatch OOP languages use: walk the linearization
most-derived-first, first definition of a name wins
(`Resolver::resolve`). This was a deliberate scope cut, not an oversight:
`dominant`/`recessive` requires tracking *which parent branch* a
definition arrived through, not just linearization order, which is a
materially different (and more complex) resolution algorithm. The
three AoC examples never needed it, so it stayed out of v1.

## 7. The type system: `List<T>` and `marry`

`List<Integer>`, `List<String>`, and `List<List<Integer>>` are three
hand-written C structs (`HbListInt`/`HbListStr`/`HbListListInt`), not a
real generic instantiated per type argument. Real generics would need
either C++-template-style monomorphization of container code per element
type (more codegen machinery for a container type system nobody was
asking to generalize) or a uniform boxed representation (the exact kind
of "one true List struct" that gets its own set of tradeoffs). Given the
three AoC examples only ever needed integers and strings, specializing
by hand was a straightforwardly smaller project, at the honest cost of
not being a general-purpose language yet.

`marry` replaced the original `Integer::parse(s)` free function during
the syntax redesign that also introduced `heir`/`descends` everywhere.
The design goal was a *uniform* cast operator rather than one bespoke
parse function per type pair, so `marry_conversion` (`codegen.rs`) is a
small, closed conversion table (`Str<->Int`, `Bool->String`, and identity)
that both `value.marry(Type)` and the bare `marry(Type)` form inside
`.map(...)` share — see `codegen::tests::marry_*` for the whole matrix
and `codegen::tests::marry_unsupported_pair_is_an_error` for what
happens outside it. Extending the table (e.g. `Bool<->Integer`) is a
one-line addition to that `match`, not a new code path.

## 8. `hapsburg-lsp` reuses the compiler, not a reimplementation

`hapsburg-lsp`'s diagnostics, hover, symbols, and go-to-definition are
all backed directly by `ferdinand`'s own `parser`/`resolve`/`codegen`
modules (`lsp/src/analysis.rs`'s `analyze()` runs the exact same
pipeline, just stopping before `cc` is ever invoked). This was a
deliberate design constraint, not a convenience: an LSP that
reimplements "what a type error looks like" separately from the
compiler will drift from it, silently, the first time either side
changes. The cost was restructuring `compiler` into a lib+bin
(`compiler/src/lib.rs`) and threading a line number through `Stmt::Heir`
specifically so local-variable hover could report a *real* inferred
type rather than a guess (see `Codegen::var_hints`,
`codegen::tests::hapsburg_display_matches_source_syntax`).

### 8.1 Degrading gracefully instead of going blank

The CLI stops at the first hard error, by design (`SPEC.md` §9) — a
single-error-at-a-time compiler is a reasonable choice for a batch tool.
It's a bad choice for an editor: while you're mid-edit, most of the file
is probably fine and one broken class shouldn't blank out hover
everywhere else. So `analyze()` deliberately does *not* mirror the CLI's
stop-at-first-error behavior: it calls `Resolver::linearize`/`resolve`
independently for every declared class regardless of whether
`check_all()` (used only to generate the diagnostic list) succeeded, and
it keeps calling `Codegen::gen_class` for every birthed class even after
one fails, collecting one diagnostic per failure instead of stopping at
the first. `analysis::tests::
one_classs_inbreeding_error_does_not_block_others_resolution` locks this
in against a shrunk version of `examples/negative/diamond_conflict.hb`:
`E`'s `InbreedingError` produces a diagnostic, but `A` and `B` still get
full resolved hover data in the same response.

### 8.2 Word-under-cursor, not a position→AST-node index

Hover/definition/completion resolve a document position by extracting
the identifier text at that column via a straightforward text scan
(`main.rs::word_at`), then looking it up by *name* — against the
resolver's class table, a static keyword/builtin table, or the nearest
preceding `var_hints` entry with a matching name and line ≤ the cursor's.
This is a real, deliberate simplification versus mapping the exact
cursor position to a specific `Expr` node in the AST: the AST currently
carries line numbers only on a handful of node kinds (`Dynasty`,
`Trait`, `Method`, `Stmt::Heir`), not full source spans on every
expression. Name-based lookup is exactly right for dynasty/trait/method/
keyword hover (those are meaningfully global by name within a file
anyway) and a reasonable approximation for locals (nearest declaration
wins) — but it means two differently-typed locals that happen to share a
name in unrelated scopes of the same file could theoretically resolve to
the wrong one. Threading real spans through the whole `Expr` enum would
fix this properly; it wasn't done because it's a much larger, more
invasive change than the value it adds for programs this size (see
`lsp/README.md`'s own "what it doesn't do" section for the same point
from the user-facing side, and §9 below).

## 9. Memory model: honest about what isn't there

The original pitch spent real ink on a reference-counted memory model
with a logged `cause_of_death` per object and a `Hemophilia` class of
leak bugs. None of that is implemented. Every `birth()` and every
intermediate allocation (`hb_list_int_push`'s growth, `hb_string_chars`'
per-character `malloc`, etc.) is a plain `malloc` with process lifetime;
nothing is ever freed. This is a genuine, deliberate simplification, not
an oversight discovered late: for the class of programs this version
targets — short-lived CLI tools that read some input, compute, print,
and exit — a leaking allocator is indistinguishable from a correct one,
and implementing real refcounting (retain/release call insertion at
every assignment and scope exit, cycle handling, the `cause_of_death`
logging) is a substantial separate project with no payoff for those
programs. It *would* matter for a long-running Hapsburg program, which
is exactly the kind of program this version isn't trying to support yet.

One concrete consequence worth knowing: `README.md`'s own performance
section found that `hb_string_chars`' one-`malloc`-per-character
representation of `List<String>` — not the dispatch mechanism — was the
actual bottleneck in a large-input benchmark. A real memory model and a
smarter string/list representation are two different projects that
happen to share a "the runtime's data representations are naive" root
cause; fixing the representation doesn't require fixing the lifetime
story, and vice versa.

## 10. Known gaps

Collected in one place, each with a pointer to where it's exercised or
could be. See [`TODO.md`](../TODO.md) for these framed as forward-looking
work items rather than a design retrospective.

- **`self` inside a `.map(|x| ...)` lambda produces a raw C error, not a
  Hapsburg diagnostic.** The Hapsburg-level type checker happily tracks
  `self`'s type inside a lambda body (`gen_map`'s lambda branch reuses
  the outer `FnCtx`'s `leaf`, which is what makes `self`-dispatch resolve
  at all), but the lambda compiles to a `static` C function that only
  takes the element parameter — no `self`. The result is `cc` failing
  with `'self' undeclared in this function`, which `ferdinand` passes
  through verbatim instead of catching earlier. Pinned by
  `examples/negative/self_in_lambda.hb` and
  `golden::self_in_lambda_currently_fails_as_a_raw_c_error` — a
  regression test for the *current* (undesirable) behavior, so a future
  fix has to deliberately update it rather than silently changing shape
  unnoticed. The clean fix is either threading `self` through as a
  second parameter when a lambda body references it, or rejecting `self`
  inside lambda bodies with a real Hapsburg-level error at `gen_map`'s
  lambda branch.
- **`heir name descends Type = expr` doesn't check `Type` against
  `expr`'s inferred type.** `Codegen::gen_stmt`'s `Stmt::Heir` arm uses
  the explicit annotation's type for the generated C declaration
  unconditionally when one is given, never comparing it against the
  initializer's own inferred type the way `birth(...)`'s `field:`
  arguments already are (`gen_birth` does check those). Confirmed by
  hand: `heir x descends String = 5` compiles to `char* x = 5;`,
  silently wrong C that only `cc` itself catches (as a warning, not an
  error, so the build still "succeeds"). The fix is a straightforward
  type-equality check next to where `birth`'s already lives; scoped out
  of this pass because it surfaced late, while writing this document,
  not because it's hard.
- **No `super`.** The very first pitch text used `super.reign()`; it
  never made it into the three AoC examples that anchored the actual
  implementation, so it was never built. Adding it is mechanical once
  you have "the leaf class's resolved method table" already computed
  (which `Codegen` does) — it needs one more piece of context threaded
  through `FnCtx`: which *ancestor* in the current method's owning
  class's position in the linearization to resolve `super` against.
- **No column spans, only line numbers**, and not even those for every
  codegen-stage error (see `SPEC.md` §9 and `lsp/README.md`). This is
  the single change that would upgrade the most things at once: precise
  diagnostic ranges, a real position→AST-node hover lookup instead of
  word-under-cursor, and eventually rename/references support in the
  LSP. Not done because it touches the lexer, every AST node, the parser,
  and every codegen error site — the largest single mechanical change
  available, for a project whose current programs are all small enough
  that line-level precision hasn't actually caused confusion yet.
- **No runtime polymorphism, no `dominant`/`recessive` traits, no
  `abdicate`/`Regency` deprecation shims, no refcounted memory model, no
  regional stdlib renaming (`Habsburg::Netherlands` etc.)** — all
  covered above or in `SPEC.md`; listed here again as one flat list
  because "what's not built yet" is exactly the kind of question this
  file exists to answer without making someone reconstruct it from
  commit history.

## 11. Testing strategy

Three layers, each catching a different class of regression:

1. **Unit tests** (`#[cfg(test)] mod tests` inside `lexer.rs`,
   `parser.rs`, `resolve.rs`, `codegen.rs`, and `lsp/src/analysis.rs`) —
   fast, focused on one function or algorithm. The C3 linearization
   tests in particular exist because a subtle bug there would silently
   corrupt the "flattening" story this whole project is about; they
   cover the textbook diamond, the classic inconsistent-MRO failure, and
   the "reinforcing" multiple-inheritance edge case that's easy to
   confuse with the genetic-diversity check (see `resolve.rs`'s test
   module comments for exactly that confusion, worked through in code).
2. **Golden/integration tests** (`compiler/tests/golden.rs`) — actually
   invoke the `ferdinand` binary, actually call `cc`, actually run the
   resulting binary with piped stdin, and check exact output against
   known Advent of Code answers. This is the layer that exercises the
   real end-to-end path (a unit test on `gen_succession` alone wouldn't
   have caught the `is_path`-on-the-wrong-node bug that briefly broke
   `Habsburg::Range::infinite()` during this project's own development —
   only running the actual compiled Day 3 binary surfaced it).
3. **Manual LSP protocol testing** — `hapsburg-lsp` doesn't yet have
   stdio-level integration tests (a scripted JSON-RPC client was used
   during development to verify hover/symbols/definition/completion end
   to end, but that script isn't checked in as an automated test). The
   `analysis::tests` module covers the pure logic (`analyze()` takes a
   `&str`, returns owned data, no I/O), which is most of what could
   actually regress; wiring an actual `tower-lsp`-level test harness
   would close the remaining gap between "the data is right" and "the
   wire protocol serializes it correctly," which the manual testing
   during development did check once but nothing currently re-checks on
   every change.

`cargo test --workspace` runs layers 1 and 2 (56 tests, as of this
writing); `.github/workflows/ci.yml` runs it — plus `cargo fmt --check`
and `cargo clippy -- -D warnings` — on every push and pull request.
