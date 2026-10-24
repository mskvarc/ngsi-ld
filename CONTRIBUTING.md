# Contributing to ngsi-ld

Thank you for helping improve `ngsi-ld`. This guide explains how to report a problem, set up the project, and get a change merged.

By taking part you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md). To report a security issue, follow the [security policy](SECURITY.md) and do not open a public issue.

## Project Principles

Every review comes back to these, so read them before you write code. The [architecture overview](docs/architecture.md) and the guides below cover them in more detail.

- **The specification decides.** The crate targets NGSI-LD v1.9.1 (ETSI GS CIM 009 v1.9.1). Every data shape, member name, operation, parameter and error type follows the specification text. A change to NGSI-LD behaviour cites the clause that justifies it.
- **The OpenAPI document is a checklist.** It is used to check that nothing is missing. No code is generated from it, and where it disagrees with the specification, the specification wins.
- **No broker gets special treatment.** No code, comment, test or example names, targets or works around a specific broker. When a broker deviates from the specification, the crate keeps following the specification.
- **The public API is a contract.** Every public item is a SemVer commitment. Invalid documents cannot be built on the strict API, and what a client sends and what a broker returns are different types.
- **Transport is optional.** The data model and the query languages compile without an HTTP client or an async runtime.
- **No panics a caller can reach, no global state, no `unsafe`.**

## Guides

- [API guidelines](docs/api-guidelines.md): the rules for everything a caller sees, from module paths to error types and serialization.
- [Code organization](docs/code-organization.md): layers, modules, and how types and functions are split.
- [Code style](docs/code-style.md): the Rust conventions, comments, documentation, examples and doc tests.
- [Live testing](docs/live-testing.md): running the examples and the opt-in tests against a real broker.

## Reporting a Bug

Open an issue with:

- the crate version and the Cargo features you enabled
- a minimal code sample that reproduces the problem
- what you expected and what happened
- if the crate disagrees with the specification, the clause and the text that says so

If a broker answers differently from the specification, report that to the broker. The crate does not change to match it.

## Proposing a Change

Open a pull request directly for a bug fix or a documentation fix.

Open an issue first for new functionality, a new public item, a new dependency or a breaking change, so the design is agreed before the code is written.

## Setting Up

The toolchain is pinned in `rust-toolchain.toml`, and `rustup` installs it on first use. Formatting needs nightly `rustfmt`, because `rustfmt.toml` uses unstable import options. The other checks need three tools.

```sh
rustup toolchain install nightly --profile minimal --component rustfmt
cargo install --locked taplo-cli cargo-deny cargo-machete
```

No feature is on by default. The features are `chrono`, `geo-types`, `geojson`, `http`, `linked-data` and `reqwest`.

## Before You Open a Pull Request

Each check below runs in CI and must pass with no warnings.

| Check | Command |
| --- | --- |
| Format | `cargo +nightly fmt --all --check` |
| TOML format | `taplo fmt --check` |
| Clippy | `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` |
| Clippy per feature | `cargo clippy --workspace --all-targets --no-default-features --features "<name>" --locked -- -D warnings`, once with no feature and once for each feature |
| Tests | `cargo test --workspace --all-features --locked` |
| Rustdoc | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked` |
| Licences, advisories, bans and sources | `cargo deny check` |
| Unused dependencies | `cargo machete --with-metadata` |

The lint levels in `Cargo.toml`, `clippy.toml`, `rustfmt.toml`, `.taplo.toml` and `deny.toml` are project policy. Fix the code, not the lint: do not lower a level or add `#[allow(...)]` or `#[expect(...)]`. If you think a lint is wrong for one specific case, say why in the pull request.

## Tests

- Every module has tests, serde code and builders included.
- Every type that crosses the API boundary has a round-trip test: serializing what was read from a document in [canonical form](docs/api-guidelines.md#serialization) gives the same JSON, with members in the same order.
- The Annex C examples ETSI publishes are golden tests under `tests/golden/`. They are read, written back and compared.
- Every client operation is tested against a mock server that checks the exact method, path, query string, headers and body the crate sent, and answers with each response the specification allows: success, each problem type, and partial results.
- Every rejection a builder promises has a test. A rejection at compile time is a `trybuild` case under `tests/ui/`.
- Live tests are `#[ignore]`d and never run in CI. See [live testing](docs/live-testing.md).

## Dependencies

- Before writing new functionality, look for a well-maintained crate that already does it.
- Every downstream build pays for a dependency. Prefer small dependency trees, set `default-features = false` and list only the features the crate uses, and put a heavy dependency behind a Cargo feature.
- Declare full versions (`1.2.3`, never `1` or `1.2`) and no range operators. Keep every dependency table in alphabetical order.
- Name a feature in kebab-case after what it enables, and enable optional dependencies with `dep:`.
- `Cargo.lock` is committed, and CI builds with `--locked`.

## Commits

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org), written in lowercase. The types are `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `style`, `build`, `ci` and `chore`. Mark a breaking change with `!`, as in `feat(model)!: rename the temporal query builder`.

The changelog is generated from commit messages with `git-cliff`, and a commit that does not follow the format is left out of it.

## Pull Requests

- Keep a pull request to one concern.
- A change to an API follows [Changing an API](docs/code-organization.md#changing-an-api): every caller is updated in the same pull request, and nothing is kept around for compatibility.
- Do not add personal identifiers (names, email addresses, handles) to code, fixtures, examples, headers or manifests.

## Licence

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed under the [Apache License, Version 2.0](LICENSE-APACHE-2.0.md) and the [MIT licence](LICENSE-MIT.md), without any additional terms or conditions.
