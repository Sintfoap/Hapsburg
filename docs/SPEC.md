# The Hapsburg Language Specification (v1)

This document specifies the Hapsburg language as implemented by `ferdinand`
in this repository. It is a specification of an *implementation*, not an
abstract standard with multiple conforming compilers in mind — where the
compiler's actual behavior and this document disagree, the compiler's
behavior is a bug in one of the two, and the fix is whichever makes them
agree without breaking `examples/`. For the *why* behind these choices, see
[`DESIGN.md`](DESIGN.md); this document is the *what*.

Every claim below is backed by a test in `compiler/src/*.rs` (`#[cfg(test)]
mod tests`) or `compiler/tests/golden.rs`. If you find a discrepancy
between this document and the compiler, the tests are the tiebreaker.

## 1. Lexical structure

### 1.1 Comments

`//` starts a line comment, running to end of line. There are no block
comments.

### 1.2 Identifiers

`[A-Za-z_][A-Za-z0-9_]*`. Identifiers separated by `::` form a *path*
(`Habsburg::Accumulator`, `AdventOfCode::Y2017::Day1::PartTwo`) — paths are
used for dynasty names and for referencing them.

### 1.3 Literals

- Integer: `[0-9]+`, parsed as a signed 64-bit value (`i64`/`int64_t`). No
  negative literals at the lexical level — `-5` is unary negation applied
  to `5`.
- String: `"..."`, with escapes `\"`, `\\`, `\n`, `\t`. Any other character
  after `\` is taken literally. Strings may contain literal (unescaped)
  newlines.
- Boolean: `true`, `false`.

### 1.4 Keywords

```
dynasty  descends  founder  trait  override  abstract  birth
succession  over  as  claim  contested  heir  return  self
true  false  and  or
```

Reserved but not exactly a keyword: `marry`, `print`, `abs`, `assassinate`,
`Integer`, `String`, `Bool`, `List`, `Void`, `Habsburg`, `Accumulator`,
`Range`, `infinite`, `up_to`, `Correspondence`, `receive_line`,
`receive_all`, `InbreedingError` are all ordinary identifiers lexically —
the compiler special-cases them by *name* during codegen (see §6), not by
tokenizing them differently. Practically this means a user-defined method
or dynasty named e.g. `print` will shadow the builtin's parse-time
recognition in confusing ways; don't do that (nothing currently stops you
at compile time — see [`DESIGN.md`](DESIGN.md) for why this wasn't worth
solving generically).

### 1.5 Punctuation and operators

```
{ } ( ) [ ] , . -> :: : = == != < > <= >= + - * / % |
```

(`:` survives only for named call arguments, `birth(X, seed: 0)` — every
*type* position uses `descends`, not `:`. See §3.)

## 2. Grammar

EBNF-ish; `{ }` means zero-or-more, `[ ]` means optional, `|` means
alternatives, terminals are quoted.

```
program        = { dynasty | stmt } ;

dynasty        = "dynasty" path [ "descends" path { "," path } ] [ "founder" ]
                  "{" { trait_decl | method_decl } "}" ;

trait_decl     = "trait" ident "descends" type [ "=" expr ] ;

method_decl    = "override" ident "(" [ param { "," param } ] ")"
                  [ "->" type ] "{" { stmt } "}" ;
                  (* a body of exactly the single statement `abstract`
                     makes the method abstract; any other body, including
                     an empty one, is concrete -- the declared `-> type`
                     (Void if omitted) is unaffected by what's in the body *)

param          = ident "descends" type ;

type           = path [ "<" type ">" ] ;

stmt           = heir_stmt | assign_stmt | expr_stmt | succession_stmt
                | claim_stmt | return_stmt | "abstract" ;

heir_stmt      = "heir" ident [ "descends" type ] [ "=" expr ] ;
assign_stmt    = expr "=" expr ;
expr_stmt      = expr ;
succession_stmt = "succession" "over" expr "as" ( ident | "_" )
                   "{" { stmt } "}" ;
claim_stmt     = "claim" expr "{" { stmt } "}"
                  [ "contested" "{" { stmt } "}" ] ;
return_stmt    = "return" [ expr ] ;

expr           = or_expr ;
or_expr        = and_expr { "or" and_expr } ;
and_expr       = equality { "and" equality } ;
equality       = comparison { ( "==" | "!=" ) comparison } ;
comparison     = additive { ( "<" | ">" | "<=" | ">=" ) additive } ;
additive       = multiplicative { ( "+" | "-" ) multiplicative } ;
multiplicative = unary { ( "*" | "/" | "%" ) unary } ;
unary          = [ "-" ] postfix ;
postfix        = primary { call_suffix | method_suffix | index_suffix } ;
call_suffix    = "(" [ arg { "," arg } ] ")" ;
method_suffix  = "." ident [ call_suffix ] ;
index_suffix   = "[" expr "]" ;

primary        = int_lit | str_lit | "true" | "false" | "self"
                | "(" expr ")" | lambda | birth_expr | path ;

lambda         = "|" ident "|" expr ;
birth_expr     = "birth" "(" path { "," arg } ")" ;
arg            = [ ident ":" ] expr ;   (* "name:" prefix = named arg *)
```

Notes not obvious from the grammar alone:

- A qualified `path` used as an expression (e.g. `Integer` inside
  `marry(Integer)`, or `Habsburg::Range::infinite`) parses through the same
  `primary` rule as any other identifier reference — there is no separate
  "type expression" grammar. Multi-segment paths become `Expr::PathExpr`;
  single-segment ones collapse to `Expr::Ident`. What's *legal* at that
  position (a real variable vs. a magic compiler-recognized name) is a
  codegen-time concern, not a parse-time one — see §6.
- Top-level `stmt`s (outside any `dynasty`) form the program's entry point
  and compile to `main()`. `self` is a compile error there (§5.7).
- Files are concatenated when `ferdinand` is given more than one: dynasty
  declarations pool into one namespace (a name declared twice, even across
  files, is a compile error) and top-level statements run in file
  argument order.

## 3. The type system

Concrete types: `Integer` (i64), `String` (owned `char*`), `Bool` (C
`int`, 0/1), `Void`, `List<Integer>`, `List<String>`, `List<List<Integer>>`,
`Habsburg::Accumulator`, and dynasty types (one per declared `dynasty`).

**There is no other `List<T>`.** `List<Bool>`, `List<YourDynasty>`, and any
other instantiation are compile errors — these three are hand-specialized
C container structs (`HbListInt`, `HbListStr`, `HbListListInt`), not real
generics. See [`DESIGN.md`](DESIGN.md) §"What's real vs. deferred".

**Every type-relating position uses `descends`**, not `:` — this is the
central design conceit carried all the way through the grammar, not just
at the class level:

| Position | Syntax |
|---|---|
| dynasty ancestry | `dynasty X descends A, B` |
| a field | `trait name descends Type [= default]` |
| a parameter | `param_name descends Type` |
| a local variable | `heir name [descends Type] = expr` |

A `heir` without an explicit `descends Type` infers its type from the
initializer expression — the same inference `type_of_annotation` /
`gen_expr` perform for every other expression in the compiler (see
`compiler/src/codegen.rs`).

### 3.1 Type inference

There is no unification or generalization — inference is a single
downward pass. Every expression's type is computed exactly once, from its
subexpressions' already-known types, in the same traversal that emits C
(`Codegen::gen_expr`). Concretely:

- Literals have fixed types (`Int`, `Str`, `Bool`).
- `self` has the type of the *leaf class currently being generated* — see
  §5, this is what makes whole-program devirtualization possible.
- A `heir`, `trait`, or parameter's type is either the explicit
  `descends Type` annotation or the initializer/no-annotation-available
  inferred type; **the two are never cross-checked against each other**
  for `heir` (see known gaps in [`DESIGN.md`](DESIGN.md)) but *are*
  checked for `trait`s provided via `birth(...)`.
- Arithmetic/comparison operators require `Integer` operands (except `==`
  /`!=`, which also accept two `String`s, compiled to `strcmp`, or two
  `Bool`s) and produce `Integer`/`Bool` respectively. `and`/`or` require
  `Bool` operands.
- Method/field access resolves against the *statically known* concrete
  type of the receiver — never a runtime tag, there isn't one.

### 3.2 `marry`: the cast operator

`value.marry(TypeName)` (or the bare `marry(TypeName)` form recognized
specifically as a `.map(...)` argument, applied per-element) is the only
type-conversion mechanism. `TypeName` must be a bare identifier naming
`Integer`, `String`, or `Bool` — not an arbitrary type expression, and not
a dynasty type.

The full legitimate-marriage table:

| From \ To | Integer | String | Bool |
|---|---|---|---|
| **Integer** | no-op | `hb_integer_to_string` | refused |
| **String** | `hb_integer_parse` | no-op | refused |
| **Bool** | refused | `hb_bool_to_string` | no-op |

Marrying a type into itself is always a no-op (the operand passes through
unchanged, no runtime call). Anything not in the table is a compile-time
error: `no legitimate marriage between X and Y`. `hb_integer_parse` calls
`strtoll`; a string with no parseable leading integer raises
`InbreedingError` *at runtime* (this is the one place a `marry` can still
fail after compiling — see §5.8).

## 4. Inheritance semantics

### 4.1 Declaration rules

- A `dynasty` with **zero** parents must be marked `founder`; a `founder`
  dynasty must have **zero** parents. Violating either is a compile error.
- A non-`founder` dynasty with **exactly one** parent compiles, but emits
  a warning ("single line of descent... consider marrying in a second
  parent").
- A non-`founder` dynasty with **two or more** parents is the unremarked,
  expected case.
- Declaring the same dynasty name twice (including across files given to
  one `ferdinand` invocation) is a compile error.
- Referencing an undeclared ancestor in `descends` is a compile error.

### 4.2 C3 linearization ("the pedigree")

Every dynasty's linearization — the order ancestors are consulted in when
resolving a trait or method name — is computed by the standard
[C3 algorithm](https://en.wikipedia.org/wiki/C3_linearization) (the same
one Python uses for `__mro__`): merge each parent's own linearization
together with the parent list itself, repeatedly taking the first
candidate head that doesn't appear in the tail of any remaining sequence.

Two consequences worth stating explicitly because they surprise people
coming from single-inheritance languages:

- **Parent order matters.** `dynasty C descends B, A` and
  `dynasty C descends A, B` are different declarations when `A` and `B`
  are related — one may linearize fine while the other is an
  `InbreedingError`, exactly mirroring Python's `class C(B, A)` vs.
  `class C(A, B)` when `B` itself subclasses `A`.
- **A linearization can fail even when every individual `descends` clause
  is well-formed.** If two ancestors are inherited in mutually
  incompatible orders (classically: `C(A, B)`, `D(B, A)`, `E(C, D)`), no
  single consistent order exists. `ferdinand` reports this as
  `InbreedingError: line N: cannot reconcile the ancestry of 'X' —
  conflicting lines of succession among ..., Pedigree so far: [...]`,
  before generating any code.

### 4.3 Trait and method resolution

For a given dynasty, walk its linearization from most-derived (itself)
to least-derived. The **first** declaration of a given trait or method
name encountered wins — this is the entirety of the override rule. There
is no `dominant`/`recessive` distinction and no way for a less-derived
declaration to "win" over a more-derived one that names the same member
(see [`DESIGN.md`](DESIGN.md) for the pitch feature this deliberately
simplifies away).

A dynasty can be `birth()`'d only if every method in its resolved table
has a concrete (non-`abstract`) body somewhere in its lineage. Otherwise:
`cannot birth 'X': 'method' has no heir to inherit it — line of succession
broken (still abstract)`.

### 4.4 The "no genetic diversity" refusal

If the program's **entire** set of declared dynasties forms one fully
connected graph under the ancestor relation (every pair of dynasties has
one as an ancestor of the other) — refused at compile time: `no genetic
diversity, refusing to compile — every dynasty in this program descends
from every other. see Alfonso XII.`

This check only runs when at least one dynasty in the program actually
declares two or more parents. A plain single-inheritance chain is
*always* a total order (hence always "fully connected" by this
definition), so without that guard every ordinary program would trip it —
see the check's doc comment in `compiler/src/resolve.rs` and
`examples/negative/no_genetic_diversity.hb` for a constructed case that
does trigger it (which, notably, is *not* a branching diamond — see the
comment there for why a genuine diamond can never trip this check).

## 5. Statement and expression semantics

### 5.1 `birth(Class, field: value, ...)`

Allocates one instance (`malloc`, never freed — see §7). For every trait
in the resolved trait table (in linearization-then-declaration order):
use the matching `field:` argument if given, else the trait's `= default`
expression if it has one, else it's a compile error (`birth(X) is missing
required trait 'name' and it has no default`). A `field:` argument naming
a trait the class doesn't have is also a compile error. `field:`
argument values are type-checked against the trait's declared type.

`birth(Habsburg::Accumulator, seed: Integer)` is handled as a special
case before the general path — it isn't a real dynasty (§6).

### 5.2 `succession over EXPR as NAME { }`

Four recognized forms for `EXPR`, matched structurally (not by static
type alone — see `Codegen::gen_succession`):

1. `Habsburg::Range::infinite()` — an unbounded C `for(;;)`. `NAME` (or a
   compiler-generated name if `_`) counts up from 0 but nothing stops it.
2. `Habsburg::Range::up_to(n)` — `for (i = 0; i < n; i++)`; `n` is
   evaluated once per **outer-loop iteration** if nested (this is how
   `examples/aoc2017/day3.hb`'s spiral step count, which grows over time,
   works correctly).
3. `LIST.indices` — indices `0..LIST.length`, `NAME` bound to the
   `Integer` index.
4. Any other `List<T>`-typed expression — foreach by value, `NAME` bound
   to each element in turn.

`Habsburg::Range::*` and `.indices` are **only** meaningful directly in
this position; using either as an ordinary expression is a compile error.

### 5.3 `claim COND { } contested { }`

Ordinary if/else; `contested` is optional. `COND` must be `Bool`.

### 5.4 `assassinate(ExceptionName, reason: "...")`

Prints `ExceptionName: reason` to stderr, prints a fixed pedigree-flavored
line, and calls `exit(1)`. `ExceptionName` is taken as a bare token (not
type-checked against anything — any identifier works, `InbreedingError`
is convention, not enforcement). There is no `catch`; every unhandled
error in a Hapsburg program is, structurally, this same abort path — see
§5.8 and §9 for the ones the *runtime itself* raises this way (bad list
index, `.marry(Integer)` on unparseable input, `max()`/`min()` of an
empty list).

### 5.5 `return`

Returns from the enclosing method (or exits `main` at top level, which
also implicitly `return 0`s after the last top-level statement). Every
generated non-`Void` function additionally gets a defensive zero-valued
`return` appended after its body, so a `claim`/`contested` that doesn't
cover every path (or a call to `hb_assassinate` the compiler doesn't
prove is `noreturn`) can't leave a C function without a `return` on some
path — see [`DESIGN.md`](DESIGN.md) for why this is a deliberate
simplification rather than real flow analysis.

### 5.6 Assignment

`target = value`. `target` may be a bare local/parameter name or a field
access (`expr.field = value`); indexed assignment (`list[i] = value`) is
not supported.

### 5.7 `self`

Always has the exact concrete type of the dynasty currently being
generated — never a supertype. This is true even for a method whose
*body* is textually inherited unchanged from an ancestor: the compiler
generates a fresh copy of that method for every concrete class, and
`self` (and therefore every `self.foo()` call inside it) refers to that
leaf class throughout. See §6 and [`DESIGN.md`](DESIGN.md) for what this
buys and what it forecloses (no runtime polymorphism).

Using `self` outside a dynasty method (i.e. in top-level/`main` code) is
a compile error.

### 5.8 Runtime-detectable failures

A program can compile cleanly and still call `hb_assassinate` at runtime,
in exactly these cases: `LIST[i]` out of bounds, `.max()`/`.min()` on an
empty `List<Integer>`, and `someString.marry(Integer)` on a string with
no parseable leading integer (`strtoll` finds nothing).
`examples/aoc2017/day2.hb`'s `PartTwo::value_of` also calls
`assassinate(InbreedingError, ...)` explicitly, by user code, when no
evenly-dividing pair exists in a row.

## 6. Standard library / compiler intrinsics

These are not user-space dynasties; they're names `Codegen` recognizes
specifically (see `compiler/src/codegen.rs`, `is_path`/`marry_conversion`
call sites). A user-defined dynasty or method that happens to share one
of these names shadows it in confusing, currently-unprevented ways.

| Name | Signature | Notes |
|---|---|---|
| `print(v)` | `(Integer or String) -> Void` | writes `v` + `\n` |
| `abs(n)` | `Integer -> Integer` | |
| `assassinate(Name, reason: s)` | `-> Void` (never returns) | §5.4 |
| `String.chars()` | `-> List<String>` | one 1-character `String` per byte |
| `String.lines()` | `-> List<String>` | split on `\n`, empty lines dropped |
| `String.split_whitespace()` | `-> List<String>` | |
| `String.length` | `-> Integer` | field-style access, `strlen` |
| `List<T>.length` | `-> Integer` | field-style |
| `List<Integer>.max()` / `.min()` | `-> Integer` | `assassinate`s on empty list |
| `List<Integer>.map(f)` / `List<String>.map(f)` | `-> List<...>` | `f` is `marry(Type)` or a `|x| expr` lambda; see §3.2, §6.1 |
| `birth(Habsburg::Accumulator, seed: n)` | | a mutable running total |
| `Habsburg::Accumulator.absorb(n)` | `Integer -> Void` | `value += n` |
| `Habsburg::Accumulator.value` | `-> Integer` | |
| `Habsburg::Range::infinite()` | | succession source only, §5.2 |
| `Habsburg::Range::up_to(n)` | | succession source only, §5.2 |
| `Habsburg::Correspondence::receive_line()` | `-> String` | one line of stdin, newline stripped, `""` at EOF |
| `Habsburg::Correspondence::receive_all()` | `-> String` | stdin to EOF, one trailing newline stripped |

### 6.1 `.map(...)`

Only defined on `List<Integer>` and `List<String>`. The argument is
either:

- `marry(TypeName)` — casts every element (§3.2); the input element type
  must have a legitimate marriage into `TypeName`.
- `|x| expr` — `x` is bound to one element; `expr` compiles to a `static`
  C helper function taking only `x`, so it may not capture any outer
  local variable. `expr`'s own type determines the result: `Integer` →
  `List<Integer>`, `List<Integer>` → `List<List<Integer>>` (used by
  `examples/aoc2017/day2.hb` to parse a line into a row of numbers within
  a per-line lambda), anything else is a compile error.

  **Known gap:** a lambda body that references `self` is accepted by the
  Hapsburg-level type checker (`self`'s type is still tracked) but the
  generated C helper function has no `self` parameter, so `cc` fails
  with `'self' undeclared` — a raw C compiler error leaking through
  instead of a clean Hapsburg diagnostic. Don't reference `self` inside
  a `.map(|x| ...)` lambda. See [`DESIGN.md`](DESIGN.md).

## 7. Memory and execution model

- Every `birth()` and every intermediate list/string allocation is
  `malloc`, with **process lifetime — nothing is ever freed.** There is
  no garbage collector and no refcounting. Fine for the short CLI
  programs this version targets; see [`DESIGN.md`](DESIGN.md) for the
  refcounted `cause_of_death` model this deliberately doesn't implement.
- `List<T>` values (`HbListInt`/`HbListStr`/`HbListListInt`) are 24-byte
  header structs (pointer + length + capacity) passed and returned **by
  value** — cheap to copy, sharing the underlying heap buffer, similar to
  a slice/fat-pointer.
- Dispatch is always a direct C function call. See §4.3/§5.7 and
  [`DESIGN.md`](DESIGN.md) for the whole-program monomorphization
  strategy that makes this possible with no vtables.

## 8. Compiler invocation

```
ferdinand <file1.hb> [file2.hb ...] -o <output> [--emit-c] [--keep-build-dir] [--show-pedigree]
```

- `-o <path>`: output binary path (default `a.out`).
- `--emit-c`: print the generated C to stdout before invoking `cc`.
- `--keep-build-dir`: don't delete the temporary directory holding the
  generated `.c`/runtime files after linking.
- `--show-pedigree`: print each birthed class's C3 linearization to
  stderr, e.g. `pedigree of 'X': X -> Parent -> Grandparent`.

Exit code 0 on success; 1 on any compile error (message to stderr) or if
`cc` itself fails.

## 9. Diagnostics reference

Every message below is a verbatim (or `{}`-templated) string from the
compiler, not paraphrased, so you can grep for it in
`compiler/src/{resolve,codegen}.rs` if you hit it.

**Compile-time errors:**

| Message | Cause |
|---|---|
| `dynasty 'X' declared more than once` | duplicate `dynasty` name |
| `'X' is declared 'founder' but lists ancestors (...)` | `founder` with `descends` |
| `'X' has no ancestors — descend from something...` | no `founder`, no `descends` |
| `'X' claims descent from unknown dynasty 'Y'` | undeclared ancestor |
| `cyclic line of succession: A -> B -> A` | a `descends` cycle |
| `InbreedingError: ... cannot reconcile the ancestry of 'X'` | C3 merge failure (§4.2) |
| `no genetic diversity, refusing to compile...` | §4.4 |
| `cannot birth 'X': 'm' has no heir to inherit it...` | `birth()` on an unresolved-abstract class |
| `birth(X) is missing required trait 'n' and it has no default` | §5.1 |
| `birth(X) has no trait 'n'` | unknown `field:` name in `birth(...)` |
| `no legitimate marriage between X and Y` | unsupported `marry` pair (§3.2) |
| `'X' is not a lineage anything can marry into` | `marry`'s target isn't Integer/String/Bool |
| various `error: X expects a/an Y` | argument-count/type mismatches on builtins |

**Compile-time warning:**

| Message | Cause |
|---|---|
| `'X' has a single line of descent — consider marrying in a second parent` | non-`founder` dynasty with exactly one parent |

**Runtime aborts** (via `hb_assassinate`, always to stderr, exit 1):

| Message | Cause |
|---|---|
| `list index out of range` | `LIST[i]` with `i` outside `0..length` |
| `max()`/`min() of an empty lineage` | `.max()`/`.min()` on an empty `List<Integer>` |
| `could not parse an Integer from this line of descent` | `.marry(Integer)` on unparseable input |
| (user-supplied `reason:`) | explicit `assassinate(...)` call in source |
