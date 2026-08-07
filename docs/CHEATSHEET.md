# Hapsburg Cheat Sheet

One page, for glancing at. Full grammar and semantics live in
[SPEC.md](SPEC.md); this is just the lookup table.

## Keywords

| Keyword | Meaning |
|---|---|
| `dynasty X descends A, B` | class declaration; multiple parents are the normal case |
| `founder` | marks a class with no parents (required if `descends` is omitted) |
| `trait name descends Type [= default]` | a field |
| `override name(...) -> Type { }` | a method; body can be `abstract` |
| `heir name [descends Type] = expr` | local variable binding |
| `birth(Class, field: value, ...)` | construct an instance |
| `succession over EXPR as NAME { }` | foreach |
| `claim COND { } contested { }` | if / else |
| `return [expr]` | return from the enclosing method |
| `self` | the current instance (always statically typed) |
| `and` | boolean and (C `&&`) |
| `or` | boolean or (C `||`) |
| `true` / `false` | Bool literals |

## Operators

| | |
|---|---|
| `+  -  *  /  %` | arithmetic (`Integer` only) |
| `==  !=` | equality — also works on two `String`s (`strcmp`) or two `Bool`s |
| `<  >  <=  >=` | comparison (`Integer` only) |
| `-x` | unary negation |
| `x.field`, `x[i]`, `x.method(...)` | field access, indexing, method call |

No compound assignment (`+=` etc.) and no ternary — `claim`/`contested`
covers that.

## Types

```
Integer               64-bit signed
String                char*, no interning
Bool                  true / false
Void                  a method's return type when omitted
List<Integer>          )
List<String>            }  the only three List<T> instantiations that exist
List<List<Integer>>    )
Habsburg::Accumulator  seed-birthed running total, .absorb()/.value
<AnyDynasty>           one type per declared `dynasty`
```

## `descends`: the one relationship

Every type-relating position in the grammar uses the same word:

```
dynasty X descends A, B { }               # class ancestry
trait name descends Type = default        # a field
name descends Type                        # a parameter
heir name descends Type = expr            # a local
```

## `marry`: the cast operator

```
value.marry(Type)          # cast a value
marry(Type)                # bare form, only inside .map(...)
```

| From \ To | Integer | String | Bool |
|---|---|---|---|
| **Integer** | no-op | ✓ | refused |
| **String** | ✓ | no-op | refused |
| **Bool** | refused | ✓ | no-op |

Anything not marked `✓`/no-op is a compile error:
`no legitimate marriage between X and Y`.

## `succession`: the only loop

```
succession over Habsburg::Range::infinite() as i { }     # unbounded
succession over Habsburg::Range::up_to(n) as i { }        # 0..n
succession over someList.indices as i { }                 # 0..length, index bound
succession over someList as item { }                      # foreach by value
```

`as _` discards the binder.

## Builtins by category

**I/O** — `print(v)`, `Habsburg::Correspondence::receive_line()`,
`Habsburg::Correspondence::receive_all()`

**Casting** — `value.marry(Type)` (see table above)

**String** — `.chars()`, `.lines()`, `.split_whitespace()`, `.length`

**List<Integer>** — `.length`, `.max()`, `.min()`, `.map(f)`, `.indices`
(succession source only)

**List<String>** — `.length`, `.map(f)`, `.indices`

**Math** — `abs(n)`

**Accumulator** — `birth(Habsburg::Accumulator, seed: n)`, `.absorb(n)`,
`.value`

**Errors** — `assassinate(ExceptionName, reason: "...")`

## Entry points

Top-level `heir`/statements outside any `dynasty` compile to `main()` and
run in the order they're written, across every file given to `ferdinand`
on one command line:

```
heir input descends String = Habsburg::Correspondence::receive_line()
heir day1 descends AdventOfCode::Y2017::Day1 = birth(AdventOfCode::Y2017::Day1, input: input)
print(day1.solve())
```

## Idioms

```
// Parse a line of digits into a List<Integer>
heir digits descends List<Integer> = self.input.chars().map(marry(Integer))

// Parse a whitespace-separated row of numbers
heir row descends List<Integer> = line.split_whitespace().map(marry(Integer))

// Running total
heir total = birth(Habsburg::Accumulator, seed: 0)
succession over digits as d {
    total.absorb(d)
}
return total.value

// Bail out with a runtime error when nothing matches
assassinate(InbreedingError, reason: "no evenly-dividing pair in row")
```

## Building and running

```
cargo build --release
./target/release/ferdinand file.hb -o out
echo "puzzle input" | ./out
```

```
ferdinand --emit-c file.hb -o out          # print the generated C first
ferdinand --show-pedigree file.hb -o out   # print each class's C3 linearization
ferdinand --keep-build-dir file.hb -o out  # don't delete the temp build dir
```

## Testing

```
cargo test --workspace                                   # 56 tests
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```
