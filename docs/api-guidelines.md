# API Guidelines

`ngsi-ld` is a published library, and its public API is a contract with every crate that depends on it. A mistake here cannot be quietly fixed later, so these rules apply to every item a caller can see.

## Ergonomics

The common case reads in a few lines: build an entity, query entities, subscribe. A caller writes no `Some(...)`, no nested newtype construction for each argument, and no positional argument lists.

- A type with more than three or four fields, or with optional fields, gets a builder. Builders are generated with `bon`, operations included: an operation's builder ends with `.send()`.
- A builder takes `impl Into<T>` or `impl TryInto<T>` where that removes ceremony without hiding a conversion that can fail.
- A builder takes names and other validated values as plain strings, so the common case reads in one line. It never panics on an invalid one: it collects every problem it finds and `build()` reports them together.
- An optional setter takes the value, not an `Option`. A caller who has an `Option` uses the builder's `maybe_` setter.

## Invalid Documents

On the strict API, a document the specification forbids cannot be built.

- A required member is enforced at compile time by the builder's typestate.
- A combination of members the specification forbids cannot be constructed.
- What can only be checked at runtime is checked in `build()`, which returns `Result`.
- A value is validated when it is created, so a validated value is never checked again.

A caller testing that a broker rejects bad input still has to be able to send it. That path is separate, clearly named, and never reachable by accident from the strict API.

## Requests and Responses

What a client sends and what a broker returns are different types. Members only a broker writes (`createdAt`, `modifiedAt`, `deletedAt`, `instanceId`, `previousValue` and the rest) never appear on a type a client builds for a request, so setting one is a compile error.

## Representations

The normalized, concise and simplified representations are distinct, named choices, never a flag on one type.

The kind of a document or an attribute is never guessed from its shape. In the normalized form it is read from the `type` member. Where the concise form omits `type`, the kind is inferred exactly as the specification's inference rules say (clauses 4.5.2.3, 4.5.3.3, 4.7.3 and their counterparts), and that inference is implemented once, in one parser. Untagged serde enums are not used for shapes the specification defines.

## Paths and Namespaces

The public API is the list of namespaces. A namespace is a public module at most two levels below the crate root, such as `model`, `model::geometry`, `query::q` or `http::operation`.

- Every public item has exactly one public path: `<namespace>::Item`, or `<namespace>::builders::Item` for a type whose name ends in `Builder`.
- The crate root re-exports nothing. There is no prelude.
- A namespace's module file re-exports, by name, the public items of the private modules beneath it. That and the `builders` module are the only places a `pub use` appears. A glob re-export, a second path to an item, and a re-export of another namespace's items or of a foreign crate's are not allowed.
- The `unreachable_pub` lint fails the build when a public item is not re-exported by its namespace.

Adding a public item is a decision, not a side effect. An item is private or `pub(crate)` unless a caller needs it.

## Exhaustive and Non-Exhaustive Types

`#[non_exhaustive]` is decided for each type, and its documentation says which applies and why.

- An enum the specification closes, such as the attribute kinds, the representations or the geometry types, is exhaustive, so a caller can match it without a wildcard.
- A type that will grow with future versions of the specification is `#[non_exhaustive]`.

## Domain Values

`String` and `&str` carry only human-readable text with no structure, such as a description or a title. Every value with domain meaning has its own type.

| Value | Type |
| --- | --- |
| URL | `url::Url` |
| IRI | `iri-rs` types |
| URN | `urn_rs::Urn` |
| Language tag | `langtag::LangTagBuf` |
| Media type | `mediatype::MediaType` |
| Timestamp, date, time | the `model` temporal types, built on `jiff` |
| Unit code | `cefact_units::UnitCode` |
| Entity id, entity type, attribute name, scope, dataset id, compact term | a validated newtype in `model` |

The rule covers parameters, return types, fields, enum payloads and map keys. A `HashMap<String, _>` is a sign that a key type is missing.

The query languages (`q`, `scopeQ`, `geoQ`, `pick` and `omit`, `csf`) are typed syntax trees with a printer and a parser, never strings a caller assembles.

JSON is held in a typed model. JSON the specification carries as opaque data, such as the value of a `JsonProperty`, is a `model::RawJson`, kept verbatim with its member order.

A value that is genuinely boolean, such as the answer of `is_empty()`, is a `bool`. Any other flag is a two-variant enum named after what it means. A type with more than one flag holds a typed set.

## Errors

Every fallible public function returns a typed error enum, defined with `thiserror`, and documents it under `# Errors`.

- A variant is designed for a caller to match on. It carries typed fields, never a `String` message, and chains its cause with `#[source]` or `#[from]`. A cause is never flattened with `to_string()`.
- A `Result` never has `String` or `&str` as its error type, and a library function never returns a type-erased error such as `Box<dyn Error>`.
- The message lives in `#[error("...")]` and is rendered from the typed fields. It is lowercase prose with no trailing punctuation, keeps the casing of spec tokens such as `Property`, interpolates through `Display`, never `Debug`, and does not cite a clause.

## Serialization

Who owns the wire format decides the casing.

- **The specification's format.** Anything that crosses the NGSI-LD API boundary uses exactly the casing ETSI GS CIM 009 v1.9.1 gives it: `Property`, `GeoProperty`, `observedAt`, `datasetId`, `keyValues`, `temporalValues` and so on. That covers request and response bodies, query parameter values, JSON-LD terms and every enum that maps to a token of the specification. When the casing looks inconsistent, cite the clause: the specification still wins.
- **The crate's own formats.** Every value the crate itself defines is kebab-case: `#[serde(rename_all = "kebab-case")]`, `#[strum(serialize_all = "kebab-case")]`. Keys keep their default casing.

If it is unclear which side a type falls on, treat it as the specification's until the specification shows otherwise.

A document is in canonical form when it is written the way the crate writes it: its members in the crate's order, and each value in its canonical spelling, such as an array where the specification also allows a single object. For an entity, the order is `id`, `type`, `scope`, `expiresAt`, the attributes in their order, `createdAt`, `modifiedAt`, `deletedAt` and then `@context`.

The canonical form is the default everywhere. A value whose spelling the crate normalizes (a URI, an identifier, a URL, a `DateTime` or `Time` fraction) also has a `verbatim` constructor that keeps the text it was given and writes it back; the spelling is not part of the value, so equality, hash and order are those of the canonical form.

Every type on the specification's side round-trips: serializing what was read from a document in canonical form yields the same JSON value, and entity members keep their order.

## HTTP

- `Content-Type` and `Accept` are `mediatype::MediaType` values, never strings.
- The `Link`, `Content-Type`, `Accept` and `NGSILD-Tenant` headers are set in one request pipeline. No operation sets them itself.

## Common Traits

A public type implements the traits a caller expects when they are meaningful for it: `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, and it is `Send` and `Sync`. This follows C-COMMON-TRAITS in the Rust API Guidelines.

## Names

Public names follow the naming conventions of the Rust API Guidelines: `as_`, `to_` and `into_` conversions, `iter`, `iter_mut` and `into_iter`, and getters without a `get_` prefix.

A constructor that returns a `Result` is prefixed `try_` (`try_new`, `try_from_*`), as `TryFrom` and the validated newtypes are.

## Process-Wide Behaviour

The crate holds no global state. It has no statics a caller cannot see, no global allocator and no process-wide configuration, and it never prints to stdout or stderr.

It compiles on the stable toolchain pinned in `rust-toolchain.toml` and on its declared `rust-version`. Nightly is used only for `rustfmt`.
