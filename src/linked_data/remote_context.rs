//! Loading the remote contexts an `@context` references (NGSI-LD clause 5.5.6, after the `LoadDocumentCallback`
//! of JSON-LD 1.1 API section 9.4).

use crate::model::RawJson;
use iri_rs::Iri;
use std::error::Error;

/// Loads the document behind the URL of a remote context.
///
/// [`ContextProcessor::process`] uses it. The core context and the contexts it imports are bundled and never reach a
/// loader.
///
/// The document is the JSON the URL serves: an object whose `@context` member is the context. A relative reference
/// inside it resolves against `url`.
///
/// ```
/// use iri_rs::Iri;
/// use ngsi_ld::{linked_data::RemoteContextLoader, model::RawJson};
/// use std::{
///     future,
///     io::{self, ErrorKind},
/// };
///
/// /// Serves one context from memory.
/// struct Fixed(RawJson);
///
/// impl RemoteContextLoader for Fixed {
///     type Error = io::Error;
///
///     fn load(&self, url: Iri<&str>) -> impl Future<Output = Result<RawJson, io::Error>> + Send {
///         let loaded = if url.as_str() == "https://example.org/context.jsonld" {
///             Ok(self.0.clone())
///         } else {
///             Err(io::Error::from(ErrorKind::NotFound))
///         };
///         future::ready(loaded)
///     }
/// }
/// ```
///
/// [`ContextProcessor::process`]: crate::linked_data::ContextProcessor::process
pub trait RemoteContextLoader: Sync {
    /// Why a document could not be loaded.
    type Error: Error + Send + Sync + 'static;

    /// Loads the document at `url`.
    fn load(&self, url: Iri<&str>) -> impl Future<Output = Result<RawJson, Self::Error>> + Send;
}
