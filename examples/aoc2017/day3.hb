// Advent of Code 2017, Day 3: Spiral Memory.
// Find a number's position in a square spiral built outward from 1 at the
// center, report the Manhattan distance back to the center.
//
// This one doesn't need clever subclassing -- Day3's own solve() just
// walks the spiral directly. The Ring/NextRing dynasties below are kept
// purely as thematic flourish (an "indulgent" flourish, as the pitch put
// it): each ring of the spiral inheriting its center point from the
// previous ring. They are declared, type-checked structurally, and never
// birthed -- which is deliberately on-theme: even a plain iterative
// algorithm gets dressed up in genealogy in Hapsburg, whether it needs it
// or not.

dynasty Puzzle::Solution founder {
    trait input: String

    override solve() -> Integer {
        abstract
    }
}

dynasty AdventOfCode::Y2017::Spiral::Ring founder {
    trait center_x: Integer = 0
    trait center_y: Integer = 0
    trait radius: Integer = 0

    override walk() -> Integer {
        abstract
    }
}

dynasty AdventOfCode::Y2017::Spiral::NextRing descends AdventOfCode::Y2017::Spiral::Ring {
    override walk() -> Integer {
        return self.radius
    }
}

dynasty AdventOfCode::Y2017::Day3 descends Puzzle::Solution {

    override solve() -> Integer {
        let target = Integer::parse(self.input)
        let x = 0
        let y = 0
        let step = 1
        let count = 1
        let dx = 1
        let dy = 0

        claim target == 1 {
            return 0
        } contested {
            // keep walking, the founder square doesn't count
        }

        succession over Habsburg::Range::infinite() as _ {
            succession over Habsburg::Range::up_to(step) as _ {
                x = x + dx
                y = y + dy
                count = count + 1

                claim count == target {
                    return abs(x) + abs(y)
                } contested {
                    // this square isn't the one, move on
                }
            }

            claim dx == 1 and dy == 0 {
                dx = 0
                dy = 1
            } contested {
                claim dx == 0 and dy == 1 {
                    dx = -1
                    dy = 0
                    step = step + 1
                } contested {
                    claim dx == -1 and dy == 0 {
                        dx = 0
                        dy = -1
                    } contested {
                        dx = 1
                        dy = 0
                        step = step + 1
                    }
                }
            }
        }

        return 0
    }
}

let day3 = birth(AdventOfCode::Y2017::Day3, input: "1")
print(day3.solve())

let day3b = birth(AdventOfCode::Y2017::Day3, input: "12")
print(day3b.solve())

let day3c = birth(AdventOfCode::Y2017::Day3, input: "23")
print(day3c.solve())

let day3d = birth(AdventOfCode::Y2017::Day3, input: "1024")
print(day3d.solve())
