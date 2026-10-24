//! Linked data: the `@context` of NGSI-LD documents applied as JSON-LD, behind the `linked-data` feature.
//!
//! NGSI-LD is linked data (clause 4.4). A name in a document is a term that the document's `@context` expands
//! to a Fully Qualified Name, so two documents written under different contexts can mean the same thing with
//! different spellings: under one context `Vehicle` may be `https://example.org/Vehicle`, under another `Car`
//! may be the same IRI (clause 5.5.7). This module processes contexts and moves names and documents between
//! them:
//!
//! - [`ContextProcessor`] reads the bundled core context once and builds [`ActiveContext`]s: processed `@context`s,
//!   with the core context always applied last, as clauses 4.4 and C.9 require, and remote contexts loaded by a
//!   [`RemoteContextLoader`];
//! - [`LinkedName`] is a name an active context expands, compacts and translates;
//! - [`LinkedDocument`] is a document that is expanded to JSON-LD and compacted back;
//! - [`Recontextualize`] is a document whose every name, query-language members included, is
//!   rewritten from one context into another.
//!
//! The core context is bundled with the crate, so processing it needs no network access. The JSON-LD processor's
//! own types stay inside the module: its errors surface as [`JsonLdError`].
//!
//! ```
//! use ngsi_ld::{
//!     linked_data::ContextProcessor,
//!     model::{AttributeName, Context, ContextEntry, InlineContext},
//! };
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> anyhow::Result<()> {
//! let context = Context::Single(ContextEntry::Inline(
//!     r#"{"speed": "https://example.org/speed"}"#.parse::<InlineContext>()?,
//! ));
//! let active = ContextProcessor::try_new()?
//!     .process_offline(&context)
//!     .await?;
//! let speed: AttributeName = "speed".parse()?;
//! assert_eq!(active.expand(&speed)?.as_str(), "https://example.org/speed");
//! # Ok(())
//! # }
//! ```
//!
//! # Contexts
//!
//! The NGSI-LD core context (clause 4.4) is bundled, together with the W3C Data Integrity v2 context its `ngsildproof`
//! term imports, so processing the core context never reaches the network.
//!
//! [`ContextProcessor`], [`ActiveContext`], [`ContextTerm`], [`RemoteContextLoader`], [`RequiresLoader`],
//! [`BundledContextError`], [`ProcessingError`].
//!
//! # Names
//!
//! [`LinkedName`], [`ExpansionError`], [`CompactionError`], [`TranslationError`].
//!
//! # Documents
//!
//! A document is expanded from its canonical normalized JSON under the active context (clause 5.5.7, after JSON-LD 1.1
//! API sections 4.1 and 4.2): an entity read in concise representation expands exactly as the same entity read in
//! normalized representation, and the document's own `@context` member is not used. Compaction writes the active
//! context's user `@context` as the document's `@context`, and the result is read normalized. The query-language
//! members (`q`, `scopeQ`, `csf`, `pick`, `omit`, ...) come through as written; [`ActiveContext::recontextualize`]
//! rewrites them from one context to another. Compaction writes a one-element array as its element, and where a data
//! type allows one value or an array (an entity's `type` and `scope`, a Relationship's `object`, a `VocabProperty`'s
//! `vocab`, a Property's `value`) nothing is put back: an entity written with `"type": ["Vehicle"]` comes back with
//! `"type": "Vehicle"`, equal as JSON-LD, not as written.
//!
//! [`LinkedDocument`], [`ExpandedDocument`], [`DocumentExpansionError`], [`DocumentCompactionError`],
//! [`Recontextualize`], [`NameMappingError`].
//!
//! # Errors
//!
//! [`JsonLdError`].

pub use crate::linked_data::{
    active_context::{ActiveContext, ContextTerm, ProcessingError},
    context_processor::ContextProcessor,
    core_context::{BundledContextError, RequiresLoader},
    document::{DocumentCompactionError, DocumentExpansionError, ExpandedDocument, LinkedDocument},
    json_ld_error::JsonLdError,
    name::{CompactionError, ExpansionError, LinkedName, TranslationError},
    recontextualize::{NameMappingError, Recontextualize},
    remote_context::RemoteContextLoader,
};

mod active_context;
mod context_processor;
mod core_context;
mod document;
#[cfg(test)]
mod fixtures;
mod json_ld_error;
mod name;
mod recontextualize;
mod remote_context;
