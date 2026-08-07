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
//
// Run it: echo 1024 | ferdinand-compiled-day3   (answer: 31)

dynasty Puzzle::Solution founder {
    trait input descends String

    override solve() -> Integer {
        abstract
    }
}

dynasty AdventOfCode::Y2017::Spiral::Ring founder {
    trait center_x descends Integer = 0
    trait center_y descends Integer = 0
    trait radius descends Integer = 0

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
        heir target descends Integer = self.input.marry(Integer)
        heir x = 0
        heir y = 0
        heir step = 1
        heir count = 1
        heir dx = 1
        heir dy = 0

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

heir puzzle_input descends String = Habsburg::Correspondence::receive_line()
heir day3 descends AdventOfCode::Y2017::Day3 = birth(AdventOfCode::Y2017::Day3, input: puzzle_input)
print(day3.solve())
