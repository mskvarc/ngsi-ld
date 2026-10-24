// SPDX-FileCopyrightText: 2014-2025 Sean McArthur
// SPDX-License-Identifier: MIT

//! The `Link` header (RFC 8288) as NGSI-LD uses it (NGSI-LD clauses 6.3.5 and 6.3.10).
//!
//! Derived from the disabled `Link` implementation in <https://github.com/hyperium/headers>. This module, including
//! local modifications, is distributed under the MIT License.
//!
//! NGSI-LD reads two kinds of link from this header: the JSON-LD `@context` link (clause 6.3.5), with
//! `rel="http://www.w3.org/ns/json-ld#context"` and `type="application/ld+json"`, and the pagination links
//! (clause 6.3.10), with `rel="next"` or `rel="prev"`. Only the `rel`, `type` and `profile` parameters are read;
//! every other parameter is ignored, as RFC 8288 section 3.4.1 requires of a recipient that does not know it.

pub(super) mod error;
pub(crate) mod header;
mod quoting;
pub(super) mod relation_type;
pub(super) mod value;
