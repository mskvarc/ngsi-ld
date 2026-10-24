# Annex C examples

These are the JSON examples from Annex C of the NGSI-LD specification, as ETSI publishes them. They come unchanged from the `examples/` directory of the [ETSI NGSI-LD OpenAPI repository](https://forge.etsi.org/rep/cim/ngsi-ld-openapi), tag `v1.8.1`, commit `e034261635afd95f161974aaf19467270216c431`. ETSI licenses them under BSD-3-Clause (see [`LICENSE.md`](LICENSE.md)), and that licence covers only the files in this directory.

They're the v1.6 edition. Their `@context` still points at the v1.6 core context, and the C.2.2 Vehicle uses none of the attribute kinds added since then. The fixtures in [`../authored/`](../authored/) cover those.

Three of them aren't valid v1.9.1 documents: `Subscription_C.4.json`, `Vehicle_temporal_scope_C.5.16.3.json` and `ContextSourceRegistration_C.3.json`. [`../etsi-corrected/`](../etsi-corrected/) has a fixed copy of each, along with what was fixed.
