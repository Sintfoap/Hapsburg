// Advent of Code 2017, Day 1: Inverse Captcha.
// Sum the digits that match the *next* digit in a circular list.
// Part 2 only changes what "next" means -- which is exactly the kind of
// change Hapsburg wants expressed as a subclass overriding one trait.
//
// Run it: echo 1122 | ferdinand-compiled-day1
// (the real puzzle input is one line shared by both parts, same as the
// actual AoC problem -- only the illustrative examples in the problem
// text differ per part)

dynasty Puzzle::Solution founder {
    trait input descends String

    override solve() -> Integer {
        abstract
    }
}

dynasty AdventOfCode::Y2017::Day1 descends Puzzle::Solution {

    override offset() -> Integer {
        return 1
    }

    override solve() -> Integer {
        heir digits descends List<Integer> = self.input.chars().map(marry(Integer))
        heir n descends Integer = digits.length
        heir total = birth(Habsburg::Accumulator, seed: 0)

        succession over digits.indices as i {
            heir current descends Integer = digits[i]
            heir heir_digit descends Integer = digits[(i + self.offset()) % n]

            claim current == heir_digit {
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

heir puzzle_input descends String = Habsburg::Correspondence::receive_line()

heir part1 descends AdventOfCode::Y2017::Day1 = birth(AdventOfCode::Y2017::Day1, input: puzzle_input)
print(part1.solve())

heir part2 descends AdventOfCode::Y2017::Day1::PartTwo = birth(AdventOfCode::Y2017::Day1::PartTwo, input: puzzle_input)
print(part2.solve())
