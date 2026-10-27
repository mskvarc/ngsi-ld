//! The operations on the `@context`s a broker stores and serves (NGSI-LD clause 5.13), bound to `jsonldContexts`
//! and `jsonldContexts/{contextId}` (clauses 6.29 and 6.30): one request type per operation and representation, and
//! how their responses are read.

pub(super) mod add;
pub(super) mod delete;
pub(super) mod list;
pub(super) mod parameters;
pub(super) mod serve;
