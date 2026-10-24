# Code Style

These are the conventions for writing Rust in `ngsi-ld`. Many of them are enforced by the lints in `Cargo.toml` and the formatter settings in `rustfmt.toml`; the rest are checked in review.

## Formatting

- Rust is formatted with `cargo +nightly fmt`, never stable `rustfmt`: `rustfmt.toml` uses unstable import options that only nightly honours.
- TOML is formatted with `taplo fmt`.

## Imports and Paths

- All `use` statements sit at the top of the file in one block, with no blank lines or groups inside it, so the formatter can sort and merge them.
- Use `crate::` paths, not `super::`.
- Import a public item through its namespace path (`crate::model::Entity`), inside the crate too. A module path below a namespace is used only for a `pub(crate)` item.
- A path of three or more segments is never written inline. Import it and use the short name. Inline paths of one or two segments, such as `url::Url` or `std::fmt`, are fine. This applies everywhere, doc examples included, and `clippy::absolute_paths` enforces it.

## Deriving Traits

A trait that can be derived is derived, not implemented by hand.

| Trait or behaviour | Use |
| --- | --- |
| `Display`, `FromStr` on enums | `strum::Display`, `strum::EnumString` |
| Variant iteration, `&'static str`, variant count | `strum::EnumIter`, `strum::IntoStaticStr`, `strum::EnumCount` |
| `Display`, `FromStr`, `Debug` on structs and newtypes | `derive_more::Display`, `derive_more::FromStr`, `derive_more::Debug` |
| `From`, `Into`, `AsRef`, `Deref` | `derive_more::From`, `derive_more::Into`, `derive_more::AsRef`, `derive_more::Deref` |
| Variant checks | `derive_more::IsVariant` |
| A validated newtype | `nutype` |
| `Error` | `thiserror::Error`, with `#[from]` or `#[source]` for causes |
| `Default` | `#[derive(Default)]`, with `#[default]` on an enum variant |
| `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash` | `#[derive(...)]` |
| A builder | `bon` |
| A serde rename or skip | `#[serde(rename_all = "...")]` at type level, `#[serde(skip_serializing_if = "Option::is_none")]` |
| A compile-time IRI | `iri_rs::iri!` |

- Derive only the traits a type actually satisfies.
- If a derive is wrong for one field, reshape the type first. If that cannot help, use `derive_where` for that field rather than writing the whole impl by hand.
- A hand-written impl of a derivable trait carries a comment that explains why no derive works. A hand-written `Serialize` or `Deserialize` is allowed only where serde attributes cannot express a shape of the specification, and says so.
- Before adding a dependency for a derive, check the derives already in use.

## Errors and Panics

- No `.unwrap()`, `.expect()`, `panic!()`, `todo!()`, `unimplemented!()` or `unreachable!()` in library code, `LazyLock` and other one-time initializers included. Propagate with `?`, use combinators, or match. The lints deny all of them, and tests may still use `unwrap`, `expect` and `panic!`.
- A value of a type that is parsed fallibly but known at compile time is built with a compile-time macro such as `iri_rs::iri!`, or an infallible `from_static` constructor, never with a parse at runtime.
- A `Result` is never ignored.
- Each module or domain defines its own error enum with `thiserror`. The [API guidelines](api-guidelines.md#errors) describe what a public error looks like.

## Match Expressions

- No wildcard arm on an enum: list every variant, and group variants that share handling with `|`. `clippy::wildcard_enum_match_arm` enforces it.
- A wildcard is only for a type with no fixed set of values, such as an integer or a string, and a named binding (`other => ...`) reads better than `_`.

## Ownership

- Avoid `.clone()`. Clone only when ownership has to move and no borrow, `Cow` or `Arc` fits, and leave a comment that says why.
- Write lifetimes out when a function returns a reference derived from more than one input, when a struct holds references, or when the borrowing is not obvious. Rely on elision only for simple cases with one reference.
- After setting up a value that needs to be mutable, shadow it as immutable:

  ```rust
  let mut names = collect_names();
  names.sort();
  let names = names;
  ```

- Order struct fields so that dependent resources drop in the right sequence. Fields drop in declaration order, locals in reverse.

## Functions and Conversions

- A function that does not depend on runtime values is a `const fn`.
- `new()` returns the concrete type name, not `Self`, unless a trait requires `Self`.
- A conversion that can fail is `TryFrom`, not `From`. A `From` impl succeeds for every input.

## Iterators

- Prefer iterator chains (`map`, `filter`, `flat_map` and the rest) to manual loops where they read better.
- Collect only when the collection is needed. `any`, `all`, `find`, `position`, `sum` and `product` replace a loop.

## Collections

- `IndexMap` where iteration order matters. NGSI-LD documents keep their member order.
- For a concurrent map, `DashMap` rather than a `Mutex<HashMap>` or `RwLock<HashMap>`. For other shared state that is mostly read, `RwLock` rather than `Mutex`.

## Macros

- Before writing the same shape of code a third time with only names or types changing, consider a `macro_rules!` macro. Use one only when a generic function, a trait or an iterator chain cannot say the same thing as clearly.
- Repetitive test cases (parameterized or table-driven) are generated with `macro_rules!`, not copied.
- A macro is documented like a public function: what it expands to and when to use it.
- A `macro_rules!` macro is defined before its first use, or exported with `#[macro_export]`. An exported macro is public API, follows the [API guidelines](api-guidelines.md), and refers to crate items through `$crate::`.
- Macros are hygienic. A macro uses an identifier from the call site only when that identifier is passed in as an argument.
- A procedural macro would live in its own `ngsi-ld-macros` crate, never in this one.
- Check a new macro's expansion with `cargo expand` for hidden clones, unwraps or panics.

## Comments

A comment says what the code cannot: why it is shaped this way, an invariant, a constraint, a clause of the specification, an edge case that is not obvious. A comment that repeats the next line in English is deleted.

- A comment states what is true now. It does not tell the history of the code: no "previously", "no longer", "the old", "was changed to", "the fix", "for now".
- A comment stands on its own. It does not point at other files, modules, issues, pull requests, other projects or brokers. A published standard, cited by title and clause (ETSI GS CIM 009 v1.9.1 clause 5.6.1, RFC 8288 section 3), is the one reference allowed, and only where it justifies the code.
- No section banners, divider lines, numbered step markers, or Arrange/Act/Assert labels.
- Write tersely. Avoid filler openers such as "This function..." or "Ensures that...", and avoid dashes used as asides: use a comma, a colon or parentheses, or split the sentence.

Design reasoning that spans modules, decisions about how to read the specification, and instructions for running tests go in `docs/`, not in a comment.

## Documentation

- Every public item has `///` documentation (`missing_docs` is denied), and so does a private item whose purpose is not obvious. Language, tone and detail are consistent across the crate.
- A public type cites the NGSI-LD clause it models, as in "NGSI-LD clause 5.2.5".
- The first paragraph of a public item's documentation is its summary: one short sentence of about 80 characters at most, without citations. A type reads as a noun phrase and a function starts with a verb. The clause follows in the next paragraph.
- A public function that returns `Result` documents `# Errors`. One that can panic documents `# Panics`, and it should not be able to.
- Every public type a caller builds has a compiling example that shows the ergonomic path.
- Documentation describes current behaviour. A doc that only restates an item's name is improved with real information (what `None` means, units, invariants, error conditions), not deleted.

## Examples and Doc Tests

The programs in `examples/` and the doc tests show a caller how to use the crate, so they are written the way a caller writes code.

- `main` returns `anyhow::Result<()>`. A doc test ends with `# Ok::<(), anyhow::Error>(())` or uses a hidden `# fn main() -> anyhow::Result<()> {`. Use `.context(...)` only where it adds something the error does not say.
- An example demonstrates, it does not verify. It prints what it does. Round trips, refusal checks and comparisons with expected output belong in `tests/`.
- A doc test shows its result the way any library's documentation does: an `assert_eq!` that shows what a call returns, or an `is_err()` that shows what an item refuses. An assert that only restates the input, or one hidden behind `#`, is not.
- An example's module documentation is one or two lines that say what it shows, and the `cargo run` line. Clause citations stay on the library's types.
- Ids are fixed and read like the specification's, such as `urn:ngsi-ld:Vehicle:A4567`. An example that talks to a broker deletes its own entities at the start and again at the end. Resources the broker names (subscriptions, registrations, snapshots, entity maps) are created without an id and read back from `Location`.
- An example may be a directory (`examples/<name>/main.rs` with modules) when it is too long for one file. Its modules serve only that example.
- Each example is self-contained. Setup such as building a client is repeated, not shared through a module.
- Examples are exempt from the rules about typed errors, strings and booleans, builders, derives and macros, clones and shadowing, documentation on private items, and duplication. The lints, the formatting, the single import block and the rules on broker neutrality, emoji and Markdown still apply.

## Tests

- A test's name describes the scenario it checks. Its body carries no step-by-step comments.
- A comment in a test is allowed only to justify an expected value (a clause, a derivation) or an ordering or concurrency constraint that is not obvious.
- Environment variables, ports and run instructions go in `docs/`, never in a test comment.

## Markdown

- One paragraph is one line. Never hard-wrap a paragraph; let the editor wrap it.
- Documentation files go in `docs/` and have kebab-case names. The exceptions are `README.md`, `CHANGELOG.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md` and the `LICENSE` files, and a `README.md` that describes the directory it sits in, such as a fixture directory.

## General

- No emoji in code, comments, documentation or commit messages. The section headings of the generated changelog are the one exception.
- No warnings. Fix every compiler, clippy and rustdoc warning, including ones the change did not introduce.
