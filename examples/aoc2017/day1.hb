// Advent of Code 2017, Day 1: Inverse Captcha.
// Sum the digits that match the *next* digit in a circular list.
// Part 2 only changes what "next" means -- which is exactly the kind of
// change Hapsburg wants expressed as a subclass overriding one trait.

dynasty Puzzle::Solution founder {
    trait input: String

    override solve() -> Integer {
        abstract
    }
}

dynasty AdventOfCode::Y2017::Day1 descends Puzzle::Solution {

    override offset() -> Integer {
        return 1
    }

    override solve() -> Integer {
        let digits = self.input.chars().map(Integer::parse)
        let n = digits.length
        let total = birth(Habsburg::Accumulator, seed: 0)

        succession over digits.indices as i {
            let current = digits[i]
            let heir = digits[(i + self.offset()) % n]

            claim current == heir {
                total.absorb(current)
            } contested {
                // No match this generation -- the trait dies out here.
            }
        }

        return total.value
    }
}

// Part 2: same royal house, different rule of succession.
// Note we only override offset() -- solve() is inherited untouched.
dynasty AdventOfCode::Y2017::Day1::PartTwo descends AdventOfCode::Y2017::Day1 {
    override offset() -> Integer {
        return self.input.length / 2
    }
}

let day1 = birth(AdventOfCode::Y2017::Day1, input: "1122")
print(day1.solve())

let day1b = birth(AdventOfCode::Y2017::Day1::PartTwo, input: "1212")
print(day1b.solve())
