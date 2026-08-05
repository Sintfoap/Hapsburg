// A program where the whole inheritance graph, once flattened, collapses
// into one straight line -- even though multiple inheritance is used.
// C descends B, A -- but B already descends A, so C isn't marrying in a
// second bloodline, just reconfirming the one it already has. Every
// dynasty here ends up comparable to every other (A -> B -> C -> D is a
// total order), so ferdinand refuses to compile: no genetic diversity.
//
// Note the parent order matters, exactly like Python's MRO rules: `C
// descends B, A` linearizes fine (B is more specific, listed first), but
// `C descends A, B` would be an InbreedingError (see diamond_conflict.hb)
// since it asks for A before B while B's own ancestry demands A after it.

dynasty A founder {
    override greet() -> String {
        abstract
    }
}

dynasty B descends A {
    override greet() -> String {
        abstract
    }
}

dynasty C descends B, A {
    override greet() -> String {
        abstract
    }
}

dynasty D descends C {
    override greet() -> String {
        abstract
    }
}
