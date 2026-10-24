//! What the strict API refuses at compile time.

#[test]
fn rejected_at_compile_time() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
}

/// The cases that name the HTTP binding, which only exists with the `http` feature, and the cases whose
/// diagnostics name a private module's type: rustc writes that path with the crate name only when the `http`
/// feature adds a second `subscription` module, so their expected output is the one of that feature.
#[cfg(feature = "http")]
#[test]
fn rejected_at_compile_time_by_the_binding() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/client/*.rs");
}
