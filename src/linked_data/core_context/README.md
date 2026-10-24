# Bundled Contexts

The crate embeds these two JSON-LD contexts with `include_str!`, so it can process the NGSI-LD core context without going to the network. Clause 4.4 makes the core context part of every `@context`. Both files are unmodified copies.

## `ngsi-ld-core-context-v1.9.jsonld`

The NGSI-LD core context, v1.9, downloaded from <https://uri.etsi.org/ngsi-ld/v1/ngsi-ld-core-context-v1.9.jsonld> on 2026-10-24. It's 13144 bytes, SHA-256 `b076a0ea02a6824acc763bb03c50d854a0820df9ecba6bc37ee5548c79dd34c4`.

ETSI licenses it under BSD-3-Clause (see [`LICENSE-ETSI.md`](LICENSE-ETSI.md)), and that licence covers only this file.

## `data-integrity-v2.jsonld`

The W3C Data Integrity v2 context, <https://w3id.org/security/data-integrity/v2>, which redirects to <https://www.w3.org/2025/credentials/vcdi/context/v2.jsonld>. Downloaded on 2026-10-20, SHA-256 `67f21e6e33a6c14e5ccfd2fc7865f7474fb71a04af7e94136cb399dfac8ae8f4`.

The core context can't be processed without it. Its `ngsildproof` term imports this context as a scoped context, and JSON-LD processing validates scoped contexts.

Copyright © 2025 World Wide Web Consortium. It's licensed under the W3C Software and Document License, 2023 version (see [`LICENSE-W3C.md`](LICENSE-W3C.md)), which covers only this file.
