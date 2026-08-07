//! Static documentation for hover/completion on keywords and compiler
//! intrinsics — the things that don't come from resolving a particular
//! document, unlike dynasty/trait/method/variable info (see analysis.rs).

pub const KEYWORDS: &[(&str, &str)] = &[
    ("dynasty", "Declares a class. Every dynasty must either list ancestors with `descends` or be marked `founder`."),
    ("descends", "The one relationship the whole language runs on: a dynasty descends from its parents, a trait descends from its type, a parameter descends from its type, a heir descends from its type. There's no other way to relate to a type anywhere in Hapsburg."),
    ("founder", "Marks a dynasty with no ancestors. Required if `descends` is omitted; a dynasty can't have neither."),
    ("trait", "A field declaration: `trait name descends Type [= default]`."),
    ("override", "A method declaration: `override name(params) -> Type { ... }`. Body may be a block or the single statement `abstract`."),
    ("abstract", "Marks a method with no implementation in this dynasty. birth()ing a class that still has an abstract method anywhere in its resolved lineage is a compile error."),
    ("birth", "Constructs an instance: `birth(Class, field: value, ...)`. Hapsburg's `new`."),
    ("succession", "Foreach loop: `succession over EXPR as NAME { }`. EXPR may be a list, `LIST.indices`, `Habsburg::Range::infinite()`, or `Habsburg::Range::up_to(n)`."),
    ("over", "Part of `succession over ... as ...`."),
    ("as", "Binds the loop variable in `succession over ... as NAME` (use `_` to discard it)."),
    ("claim", "If: `claim COND { } contested { }`."),
    ("contested", "The else branch of `claim`."),
    ("heir", "Local variable binding: `heir name = expr` or `heir name descends Type = expr` — an heir, descending from Type, given a value. Hapsburg's `let`, in keeping with everything else."),
    ("return", "Returns from the enclosing method."),
    ("self", "The current instance. Its concrete type is always statically known at compile time, which is what lets every method call in the generated code be a direct call — see the main README's 'runtime polymorphism' note."),
    ("and", "Boolean and (compiles to C `&&`)."),
    ("or", "Boolean or (compiles to C `||`)."),
    ("true", "Bool literal."),
    ("false", "Bool literal."),
];

pub const BUILTINS: &[(&str, &str)] = &[
    ("print", "`print(value)`: writes an Integer or String followed by a newline."),
    ("abs", "`abs(n: Integer) -> Integer`"),
    ("assassinate", "`assassinate(ExceptionName, reason: String)`: terminates with a runtime error. Conventionally thrown as `InbreedingError`."),
    ("marry", "Hapsburg's type-cast operator: `value.marry(Type)` marries `value` into a new lineage. Marrying your own type is a no-op; String<->Integer and Bool->String are the legitimate matches, everything else is refused at compile time (`no legitimate marriage between X and Y`). Also usable bare as `marry(Type)` inside `.map(...)` to cast every element."),
    ("Integer", "Built-in type. Cast into it with `someString.marry(Integer)`."),
    ("String", "Built-in type. Methods: `.chars() -> List<String>`, `.lines() -> List<String>`, `.split_whitespace() -> List<String>`, `.length -> Integer`. Cast into it with `n.marry(String)`."),
    ("Bool", "Built-in type."),
    ("List", "Built-in generic type. Only `List<Integer>`, `List<String>`, and `List<List<Integer>>` are supported (hand-specialized containers, not real generics). Methods: `.length`, `.max()`/`.min()` (List<Integer>), `.map(...)`, `.indices` (succession source only)."),
    ("Void", "The absence of a return value."),
    ("Habsburg", "Namespace for compiler intrinsics: `Habsburg::Accumulator`, `Habsburg::Range`, `Habsburg::Correspondence`."),
    ("Accumulator", "`birth(Habsburg::Accumulator, seed: Integer)`: a running total. `.absorb(n: Integer)` adds to it, `.value` reads it."),
    ("Range", "`Habsburg::Range::infinite()` / `Habsburg::Range::up_to(n)`: only valid directly as a `succession over` source, not as a storable value."),
    ("infinite", "`Habsburg::Range::infinite()`: an unbounded succession source."),
    ("up_to", "`Habsburg::Range::up_to(n)`: a succession source over `0..n`."),
    ("Correspondence", "Namespace for reading stdin: `Habsburg::Correspondence::receive_line()` (one line, no trailing newline) and `receive_all()` (everything until EOF)."),
    ("receive_line", "`Habsburg::Correspondence::receive_line() -> String`: reads one line from stdin, trailing newline stripped. `\"\"` at EOF."),
    ("receive_all", "`Habsburg::Correspondence::receive_all() -> String`: reads everything from stdin until EOF, newlines preserved (a single trailing newline is stripped)."),
    ("InbreedingError", "The conventional exception name passed to `assassinate(...)`. Also what the compiler itself reports when C3 linearization fails on a genuine diamond conflict."),
];
