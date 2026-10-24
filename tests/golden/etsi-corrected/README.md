# Corrected Annex C Examples

Fixed copies of three examples from [`../etsi/`](../etsi/). Each one parses as a valid v1.9.1 document and writes back unchanged. All three also point `@context` at the v1.9 core context instead of v1.6.

They're modified ETSI files, so they stay under ETSI's BSD-3-Clause licence (see [`LICENSE.md`](LICENSE.md)), which covers only this directory.

## `subscription-c4.json`

From `Subscription_C.4.json`, the subscription in Annex C.4.

- The `geoQ` coordinates are swapped to `[100, -1]`. The original puts the point at latitude 100, which isn't a WGS 84 position (IETF RFC 7946, section 4).
- `format` is `simplified` instead of `keyValues`. The two are synonyms, and `simplified` is the canonical spelling (Table 5.2.14.1-1).

## `temporal-scope-c516.json`

From `Vehicle_temporal_scope_C.5.16.3.json`, the temporal scope query answer in Annex C.5.16.

- The first entity's `scope` is now an array of `[scope, time]` pairs, as clause 4.5.9 requires. The v1.9.1 text of Annex C.5.16.3 writes it the same way.

## `registration-c3.json`

From `ContextSourceRegistration_C.3.json`, the context source registration in Annex C.3.

- The open interval is called `managementInterval`, as in the v1.9.1 text of Annex C.3. The original calls it `timestamp`, which Table 5.2.9-1 no longer has.
- `startAt` no longer starts with a space. No DateTime does (clause 4.6.3).
- `endpoint` ends in `/`, because an empty URL path normalizes to `/`.
- Each `EntityInfo` puts `type` before `id` and `idPattern`, which is the crate's canonical order.
