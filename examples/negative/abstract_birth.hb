// birth() on a dynasty that never received a concrete override for one of
// its abstract methods should fail to compile -- there's no heir to
// inherit the behavior from.

dynasty Puzzle::Solution founder {
    trait input descends String

    override solve() -> Integer {
        abstract
    }
}

heir doomed descends Puzzle::Solution = birth(Puzzle::Solution, input: "irrelevant")
print(doomed.solve())
