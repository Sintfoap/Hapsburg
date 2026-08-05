//! Static documentation for hover/completion on keywords and compiler
//! intrinsics — the things that don't come from resolving a particular
//! document, unlike dynasty/trait/method/variable info (see analysis.rs).

pub const KEYWORDS: &[(&str, &str)] = &[
    ("dynasty", "Declares a class. Every dynasty must either list ancestors with `descends` or be marked `founder`."),
    ("descends", "Lists this dynasty's parent(s). Multiple parents are the normal case — conflicts are resolved via C3 linearization at compile time, not picked arbitrarily."),
    ("founder", "Marks a dynasty with no ancestors. Required if `descends` is omitted; a dynasty can't have neither."),
    ("trait", "A field declaration: `trait name: Type [= default]`."),
    ("override", "A method declaration: `override name(params) -> Type { ... }`. Body may be a block or the single statement `abstract`."),
    ("abstract", "Marks a method with no implementation in this dynasty. birth()ing a class that still has an abstract method anywhere in its resolved lineage is a compile error."),
    ("birth", "Constructs an instance: `birth(Class, field: value, ...)`. Hapsburg's `new`."),
    ("succession", "Foreach loop: `succession over EXPR as NAME { }`. EXPR may be a list, `LIST.indices`, `Habsburg::Range::infinite()`, or `Habsburg::Range::up_to(n)`."),
    ("over", "Part of `succession over ... as ...`."),
    ("as", "Binds the loop variable in `succession over ... as NAME` (use `_` to discard it)."),
    ("claim", "If: `claim COND { } contested { }`."),
    ("contested", "The else branch of `claim`."),
    ("let", "Local variable binding: `let name = expr` or `let name: Type = expr`."),
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
    ("Integer", "Built-in type. `Integer::parse(s: String) -> Integer` parses a String."),
    ("String", "Built-in type. Methods: `.chars() -> List<String>`, `.lines() -> List<String>`, `.split_whitespace() -> List<String>`, `.length -> Integer`."),
    ("Bool", "Built-in type."),
    ("List", "Built-in generic type. Only `List<Integer>`, `List<String>`, and `List<List<Integer>>` are supported (hand-specialized containers, not real generics). Methods: `.length`, `.max()`/`.min()` (List<Integer>), `.map(...)`, `.indices` (succession source only)."),
    ("Void", "The absence of a return value."),
    ("Habsburg", "Namespace for compiler intrinsics: `Habsburg::Accumulator`, `Habsburg::Range`."),
    ("Accumulator", "`birth(Habsburg::Accumulator, seed: Integer)`: a running total. `.absorb(n: Integer)` adds to it, `.value` reads it."),
    ("Range", "`Habsburg::Range::infinite()` / `Habsburg::Range::up_to(n)`: only valid directly as a `succession over` source, not as a storable value."),
    ("infinite", "`Habsburg::Range::infinite()`: an unbounded succession source."),
    ("up_to", "`Habsburg::Range::up_to(n)`: a succession source over `0..n`."),
    ("InbreedingError", "The conventional exception name passed to `assassinate(...)`. Also what the compiler itself reports when C3 linearization fails on a genuine diamond conflict."),
];
