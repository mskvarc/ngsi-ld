# Code Organization

These rules decide where code lives. Code in the wrong place is a bug, not a matter of style: it ties together things that change for different reasons, and the cost of separating them grows with every caller.

## Layers

The crate is a single package split into layers, and each layer depends only on the ones below it. The [architecture overview](architecture.md) describes them.

- The data model knows nothing about HTTP.
- The query languages know nothing about transport.
- The client knows nothing about how a document is built beyond its public types.

A function or type that crosses a layer without a deliberate, justified abstraction is wrong, however many lines it saves.

## Modules

Each module owns exactly one responsibility.

- A public module at most two levels below the crate root is a namespace. The modules beneath a namespace are private by default, `pub(super)` where the namespace re-exports from a grandchild, and `pub(crate)` only where another namespace uses their `pub(crate)` items.
- A namespace's `//!` documentation is its index. It groups the namespace's items by concept under headed sections, each item an intra-doc link. A `builders` module's documentation is one line and the list of its builders.
- Documentation for callers lives on the items and in that index, never only in the documentation of a private file.
- Unit tests live in a `tests` child module of the module they test.

## Files

A file holds one cohesive concept: one type and its implementations, or one family of small, closely related types. When a file starts to host a second, unrelated concern, that concern moves to its own file right away, not once it grows. Do not grow a file by adding to the end because there is room there.

Not counting `#[cfg(test)]` code, a file past about 400 lines is a signal to split it, and a file past about 600 lines gets split.

A module is named after the thing it owns. A name that describes no domain concept, such as `utils`, `helpers`, `common`, `shared`, `misc`, `core`, `prelude`, or `types` used as a catch-all, means the module owns nothing, and it does not exist.

## Types, Traits and Functions

A struct models one concept and holds only the fields that concept owns.

- Do not thread one struct through the call graph so that callees can reach whatever they need. A function takes the values it needs, or the small struct that groups them.
- When a type's fields fall into groups that never interact, it is two or three types. Split it, and let the compiler enforce which code touches what.
- An enum or struct does not collect variants or fields from unrelated concepts.
- A name says what the thing is: the client, the entity document, the subscription request, the query. `App`, `Manager`, `Handler`, `Helper`, `Data` and `Info` on their own describe nothing.

A trait declares one capability, not a list of loosely related methods that force an implementor to stub half of them.

A function does one thing at one level of abstraction. A function with several phases gets each phase extracted into a named function.

The test is reasons to change, not line count: if two parts of a file, a type or a function change for unrelated reasons, they do not belong together.

## Duplication

Code is never copied between modules. When two unrelated places need the same behaviour, it moves into the module that owns that concept, and both use it from there.

## Changing an API

Changing an API means changing every one of its callers in the same change: every call site, import, test, doc example and example program, until the crate builds clean with no warnings.

None of these are used to soften a change:

- a deprecated alias, a `#[deprecated]` stub, or a type alias that keeps an old name alive
- a wrapper function that forwards to a renamed one, or a newtype whose only job is to connect two APIs
- an adapter, a compatibility layer, or a second path to an item
- an old code path kept reachable behind a feature, a configuration switch or a `cfg`
- a decoder that silently accepts an old field name, variant or wire format alongside the new one
- a default or an `unwrap_or` that invents a value to cover a mismatch between two APIs

A shape the specification itself requires the crate to accept is a feature, not a compatibility path.

Old code is deleted, not kept around. A breaking change waits for the next major version, where it is made directly and recorded in the changelog.
