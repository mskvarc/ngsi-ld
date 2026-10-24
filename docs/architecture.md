# Architecture

`ngsi-ld` is the client side of NGSI-LD v1.9.1 (ETSI GS CIM 009): its documents, its query languages and every operation of its HTTP API. It isn't a broker, and it isn't an MQTT client, although a notification receiver or a context source can use its types.

## The Specification Decides

Where the specification and anything else disagree, the specification wins. The OpenAPI document is only a checklist, and no code is generated from it. The golden tests use the Annex C examples ETSI publishes. The crate doesn't work around any broker: the decoder is lenient only where the specification leaves room. An operation that runs into a broker's deviation fails, so only conformant brokers are supported. Handling the known quirks of a named broker is planned, but its design isn't settled yet.

## Requests and Responses Are Separate Types

Every entity and attribute type has a direction. You send `Input` and `Patch` documents and get `Output` documents back. Members only a broker writes, like `createdAt` or `instanceId`, exist on `Output` alone, so a request can't carry them. The NGSI-LD Null is limited the same way. `into_input()` turns a retrieved entity into one you can send back.

## Representations

Normalized and concise lose nothing, so one `Entity` type reads and writes both. Simplified keeps only values, so it is a separate type, `SimplifiedEntity`, and a simplified query returns it. GeoJSON wraps these types in a `Feature`. A projection that drops `id` or `type` gives you `Entity<Output, Projected>`.

## Validation

Validated types can't hold invalid values. Builders take plain strings and numbers, refuse to compile without required members, and report every remaining problem at once from `build()`.

## Invalid Documents on Purpose

To test that a broker rejects bad input, the `http` module has a separate path that starts from a valid document or request and lets you break it. `client.send_unchecked` sends it. The strict API never accepts these values.

## Layers

`model` is the data model, and `query` adds the query languages. `subscription`, `registration` and `snapshot` hold those documents. The `http` feature adds the HTTP binding and the client, and `linked-data` adds JSON-LD processing. Lower layers never depend on higher ones, so the data model knows nothing about HTTP.

All headers are set in one request pipeline. Every public item has exactly one path, `<module>::Item` or `<module>::builders::Item`, and the crate root re-exports nothing.

## What We Left Out

- Code generated from the OpenAPI document.
- One type for both requests and responses.
- Guessing an attribute's kind from its shape.
- Strings for structured values like `q`, unit codes, dataset ids or timestamps.
- A flag that switches the representation.
- Optional setters that take an `Option`.
- Headers set separately by each operation.
- A required HTTP client or async runtime.
- A prelude or re-exports at the crate root.
- Errors as strings.
