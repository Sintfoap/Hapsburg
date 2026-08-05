// Advent of Code 2017, Day 2: Corruption Checksum.
// Part 1: for each row, sum (max - min). Part 2: for each row, find the
// one pair where one number divides the other evenly, sum the quotients.
// These are genuinely different strategies, not small variations of each
// other -- so instead of inheriting from each other, both descend from a
// shared abstract RowStrategy: siblings, not parent-and-child.

dynasty Puzzle::Solution founder {
    trait input: String

    override solve() -> Integer {
        abstract
    }
}

dynasty AdventOfCode::Y2017::RowStrategy founder {
    override value_of(row: List<Integer>) -> Integer {
        abstract
    }
}

dynasty AdventOfCode::Y2017::Day2::PartOne descends Puzzle::Solution, AdventOfCode::Y2017::RowStrategy {

    override value_of(row: List<Integer>) -> Integer {
        return row.max() - row.min()
    }

    override solve() -> Integer {
        let rows = self.input.lines().map(|l| l.split_whitespace().map(Integer::parse))
        let total = birth(Habsburg::Accumulator, seed: 0)

        succession over rows as row {
            total.absorb(self.value_of(row))
        }

        return total.value
    }
}

dynasty AdventOfCode::Y2017::Day2::PartTwo descends Puzzle::Solution, AdventOfCode::Y2017::RowStrategy {

    override value_of(row: List<Integer>) -> Integer {
        succession over row as a {
            succession over row as b {
                claim a != b and a % b == 0 {
                    return a / b
                } contested {
                    // not a legitimate pair, keep looking
                }
            }
        }
        // No valid pair found -- this row's lineage is void.
        assassinate(InbreedingError, reason: "no evenly-dividing pair in row")
    }

    override solve() -> Integer {
        let rows = self.input.lines().map(|l| l.split_whitespace().map(Integer::parse))
        let total = birth(Habsburg::Accumulator, seed: 0)

        succession over rows as row {
            total.absorb(self.value_of(row))
        }

        return total.value
    }
}

let day2a = birth(AdventOfCode::Y2017::Day2::PartOne, input: "5 1 9 5\n7 5 3\n2 4 6 8")
print(day2a.solve())

let day2b = birth(AdventOfCode::Y2017::Day2::PartTwo, input: "5 9 2 8\n9 4 7 3\n3 8 6 5")
print(day2b.solve())
