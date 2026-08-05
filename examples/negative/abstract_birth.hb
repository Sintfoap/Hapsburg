// birth() on a dynasty that never received a concrete override for one of
// its abstract methods should fail to compile -- there's no heir to
// inherit the behavior from.

dynasty Puzzle::Solution founder {
    trait input: String

    override solve() -> Integer {
        abstract
    }
}

let doomed = birth(Puzzle::Solution, input: "irrelevant")
print(doomed.solve())
