//! End-to-end tests: actually run `ferdinand` (the real binary, via `cc`)
//! against the example programs and check what comes out, instead of
//! testing internal compiler state. This is the test suite that would
//! have caught the `is_path` bug on `Habsburg::Range::infinite()` during
//! this project's own development (a unit test on `gen_succession` alone
//! wouldn't have exercised the actual CLI -> cc -> run path).

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

fn repo_root() -> PathBuf {
    // Integration tests run with CWD = the package dir (compiler/).
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A unique-per-call temp path, so tests running in parallel (cargo test's
/// default) never race on the same output file.
fn temp_output(name: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("hb-golden-{}-{}-{}", std::process::id(), n, name))
}

fn compile(src_rel: &str, name: &str) -> PathBuf {
    let src = repo_root().join(src_rel);
    let out = temp_output(name);
    let output = Command::new(env!("CARGO_BIN_EXE_ferdinand"))
        .arg(&src)
        .arg("-o")
        .arg(&out)
        .output()
        .expect("failed to invoke ferdinand");
    assert!(
        output.status.success(),
        "ferdinand failed to compile {}:\n{}",
        src_rel,
        String::from_utf8_lossy(&output.stderr)
    );
    out
}

fn run_with_stdin(bin: &PathBuf, input: &str) -> String {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to run compiled binary");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "compiled binary exited non-zero:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("program output should be valid UTF-8")
}

fn compile_expect_failure(src_rel: &str, name: &str) -> String {
    let src = repo_root().join(src_rel);
    let out = temp_output(name);
    let output = Command::new(env!("CARGO_BIN_EXE_ferdinand"))
        .arg(&src)
        .arg("-o")
        .arg(&out)
        .output()
        .expect("failed to invoke ferdinand");
    assert!(
        !output.status.success(),
        "{} was expected to fail to compile but succeeded",
        src_rel
    );
    String::from_utf8(output.stderr).unwrap()
}

#[test]
fn day1_matches_known_advent_of_code_answers() {
    let bin = compile("examples/aoc2017/day1.hb", "day1");
    assert_eq!(run_with_stdin(&bin, "1122\n"), "3\n0\n");
    assert_eq!(run_with_stdin(&bin, "1212\n"), "0\n6\n");
}

#[test]
fn day2_matches_known_advent_of_code_answers() {
    let bin = compile("examples/aoc2017/day2.hb", "day2");
    let out = run_with_stdin(&bin, "5 9 2 8\n9 4 7 3\n3 8 6 5\n");
    assert_eq!(out, "18\n9\n");
}

#[test]
fn day3_matches_known_advent_of_code_answers() {
    let bin = compile("examples/aoc2017/day3.hb", "day3");
    let cases = [
        ("1\n", "0\n"),
        ("12\n", "3\n"),
        ("23\n", "2\n"),
        ("1024\n", "31\n"),
    ];
    for (input, expected) in cases {
        assert_eq!(run_with_stdin(&bin, input), expected, "input {:?}", input);
    }
}

#[test]
fn abstract_birth_is_rejected_with_the_right_message() {
    let err = compile_expect_failure("examples/negative/abstract_birth.hb", "abstract");
    assert!(err.contains("still abstract"), "unexpected error: {}", err);
}

#[test]
fn diamond_conflict_is_rejected_as_inbreeding_error() {
    let err = compile_expect_failure("examples/negative/diamond_conflict.hb", "diamond");
    assert!(err.contains("InbreedingError"), "unexpected error: {}", err);
}

#[test]
fn no_genetic_diversity_is_rejected() {
    let err = compile_expect_failure("examples/negative/no_genetic_diversity.hb", "diversity");
    assert!(
        err.contains("no genetic diversity"),
        "unexpected error: {}",
        err
    );
}

/// Pins the known gap documented in docs/ARCHITECTURE.md: `self` inside a
/// `.map(|x| ...)` lambda isn't caught at the Hapsburg level, so it
/// surfaces as a raw C compiler error instead of a clean diagnostic.
/// This test exists so that gap stays exactly as understood rather than
/// silently changing shape (for better or worse) unnoticed.
#[test]
fn self_in_lambda_currently_fails_as_a_raw_c_error() {
    let err = compile_expect_failure("examples/negative/self_in_lambda.hb", "self_lambda");
    assert!(
        err.contains("'self' undeclared"),
        "expected the known raw-C-error failure mode, got: {}",
        err
    );
}

#[test]
fn show_pedigree_prints_the_linearization() {
    let src = repo_root().join("examples/aoc2017/day1.hb");
    let out = temp_output("pedigree");
    let output = Command::new(env!("CARGO_BIN_EXE_ferdinand"))
        .arg(&src)
        .arg("-o")
        .arg(&out)
        .arg("--show-pedigree")
        .output()
        .expect("failed to invoke ferdinand");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "AdventOfCode::Y2017::Day1::PartTwo -> AdventOfCode::Y2017::Day1 -> Puzzle::Solution"
        ),
        "unexpected --show-pedigree output:\n{}",
        stderr
    );
}
