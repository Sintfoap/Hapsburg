// A classic inconsistent-MRO diamond: C prefers A before B, D prefers B
// before A, and E tries to inherit from both orderings at once. There is
// no single consistent line of succession -- ferdinand should refuse to
// linearize E and throw InbreedingError at compile time.

dynasty A founder {
    override common() -> Integer {
        return 1
    }
}

dynasty B founder {
    override common() -> Integer {
        return 2
    }
}

dynasty C descends A, B {
}

dynasty D descends B, A {
}

dynasty E descends C, D {
}
