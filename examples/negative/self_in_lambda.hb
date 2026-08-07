// Known gap (see docs/ARCHITECTURE.md "Known gaps"): a `.map(|x| ...)` lambda
// compiles to a `static` C helper function that only takes the element
// parameter -- there's no `self` in scope there at the C level, even
// though the Hapsburg-level type checker happily tracks `self`'s type
// inside the lambda body (since FnCtx's `leaf` carries over from the
// enclosing method). The result is a raw C compiler error leaking
// through instead of a clean Hapsburg diagnostic. This file exists so
// that gap has a regression test pinning its current (undesirable but
// at least well-understood) behavior, rather than just a claim in a
// design doc nobody re-checks.

dynasty Scratch::Test founder {
    trait offset descends Integer

    override solve() -> List<Integer> {
        heir xs descends List<String> = "1 2 3".split_whitespace()
        return xs.map(|s| s.marry(Integer) + self.offset)
    }
}

heir t descends Scratch::Test = birth(Scratch::Test, offset: 10)
