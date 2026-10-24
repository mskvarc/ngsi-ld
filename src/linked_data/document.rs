//! Documents as JSON-LD: expansion of a document under an active context, and compaction of an expanded document
//! back into a document of the crate (NGSI-LD clause 5.5.7, after JSON-LD 1.1 API sections 4.1 and 4.2).
//!
//! The crate's readers follow the data types of the specification, so the arrays a data type requires are put back
//! before a compacted document is read, and an identifier is read as compaction writes it, under a prefix the
//! context defines as a compact IRI.

use crate::{
    linked_data::{
        ActiveContext,
        JsonLdError,
        core_context::{CoreContextLoader, OfflineLoader},
    },
    model::{RawJson, keyword::CONTEXT},
    seal::Seal,
};
use iri_rs::IriBuf;
use jsonld::{
    BlankIdBuf,
    Expand,
    compaction::{CompactFragment, Options as CompactionOptions},
    expansion::Options as ExpansionOptions,
    rdfx::vocabulary::{no_vocabulary, no_vocabulary_mut},
    syntax::IntoJsonWithContext,
};
use jstrict::{Print, Value as Json};

mod discovery;
mod entity;
mod query;
mod registration;
mod shape;
mod snapshot;
mod source;
mod subscription;

/// A document in expanded JSON-LD form (JSON-LD 1.1 API section 4.1): every term replaced by its Fully Qualified
/// Name, every value an array of value or node objects.
///
/// It is made by [`ActiveContext::expand_document`] and compacted back by [`ActiveContext::compact_document`],
/// under the same active context or another. Two expansions are equal when they hold the same JSON-LD, whatever
/// the order of their members.
///
/// ```
/// use ngsi_ld::{linked_data::ContextProcessor, model::{Input, Entity, Lossless}};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> anyhow::Result<()> {
/// let core = ContextProcessor::new()?.core().await?;
/// let concise = Entity::<Input>::parse(r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":55}"#, Lossless::Concise)?;
/// let normalized = Entity::<Input>::parse(
///     r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":{"type":"Property","value":55}}"#,
///     Lossless::Normalized,
/// )?;
/// assert_eq!(core.expand_document(&concise).await?, core.expand_document(&normalized).await?);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandedDocument(jsonld::ExpandedDocument<IriBuf, BlankIdBuf>);

impl ExpandedDocument {
    /// Writes the expanded document as JSON: an array of node objects.
    #[must_use]
    pub fn render(&self) -> RawJson {
        // Clone: the JSON-LD processor writes a document by consuming it, and the expansion stays the caller's.
        RawJson::from_json(self.0.clone().into_json_with(no_vocabulary()))
    }
}

/// Why a document could not be expanded.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DocumentExpansionError {
    /// The JSON-LD expansion algorithm failed on the document.
    #[error("the document could not be expanded")]
    Expansion(#[source] JsonLdError),
}

/// Why an expanded document could not be compacted into a document of type `D`; `E` is the error of `D`'s reader.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DocumentCompactionError<E> {
    /// The JSON-LD compaction algorithm failed on the expanded document.
    #[error("the expanded document could not be compacted")]
    Compaction(#[source] JsonLdError),
    /// The expanded document holds no node object, or several, so it does not compact to one document.
    #[error("the expanded document does not compact to one JSON object")]
    NotAnObject,
    /// The compacted document is not a valid document of the type asked for.
    #[error("the compacted document is not a valid document")]
    Document(#[source] E),
}

/// The hooks every [`LinkedDocument`] provides.
mod sealed {
    use crate::{linked_data::document::shape::Shape, model::name::mapping::CarriesContext, seal::Seal};
    use jstrict::Value as Json;

    /// How a document is written for expansion and read back after compaction; compaction then gives it the
    /// active context's `@context`.
    pub trait Sealed: CarriesContext + Sized {
        /// Why the document's reader refuses a JSON value.
        type ReadError;

        /// The members the document's data type writes as arrays.
        const SHAPE: Shape;

        /// Writes the document in its canonical form, normalized for an entity.
        fn write(&self, seal: Seal) -> Json;

        /// Reads the document from a compacted JSON value without `@context`.
        ///
        /// # Errors
        ///
        /// Returns the reader's error when the value is not a valid document.
        fn read(seal: Seal, json: Json) -> Result<Self, Self::ReadError>;
    }
}

/// A document that [`ActiveContext::expand_document`] expands and [`ActiveContext::compact_document`] compacts:
/// every document of the crate that is read and written in one canonical form, which is every document but the
/// fragments (written, never read), an MQTT notification (a transport envelope around one) and a local `@context`
/// (a context, not linked data).
///
/// A compaction that yields an invalid document reports the error of the document's reader: a [`DocumentError`]
/// for the documents of the data model, a [`DocumentError`] of
/// [`ReadErrorKind`](crate::subscription::ReadErrorKind) for subscriptions, registrations, snapshots and
/// their notifications, and the error of the query language for an entity selector, a geoquery or a temporal
/// query.
///
/// The trait is sealed.
///
/// [`DocumentError`]: crate::model::DocumentError
pub trait LinkedDocument: sealed::Sealed {}

impl<D: sealed::Sealed> LinkedDocument for D {}

/// Writes `document` without its `@context` member, which the active context replaces.
fn without_context(document: Json) -> Json {
    match document {
        Json::Object(object) => Json::Object(object.into_iter().filter(|entry| entry.key.as_str() != CONTEXT).collect()),
        other @ (Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Array(_)) => other,
    }
}

impl ActiveContext {
    /// Expands `document` under this active context (JSON-LD 1.1 API section 4.1), ignoring the document's own
    /// `@context`.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentExpansionError::Expansion`] when the JSON-LD expansion algorithm fails, for instance on a
    /// value that is not valid JSON-LD.
    pub fn expand_document<'a, D: LinkedDocument>(
        &'a self,
        document: &D,
    ) -> impl Future<Output = Result<ExpandedDocument, DocumentExpansionError>> + Send + use<'a, D> {
        // The document is written before the expansion starts, so the future holds its JSON, not the document.
        let json = without_context(document.write(Seal::new()));
        async move {
            let loader = CoreContextLoader::new(self.bundled(), &OfflineLoader);
            // Clone: the expansion algorithm takes its initial active context by value.
            let initial = self.processed().clone();
            json.expand_full(no_vocabulary_mut(), initial, None, &loader, ExpansionOptions::default(), ())
                .await
                .map(ExpandedDocument)
                .map_err(|error| DocumentExpansionError::Expansion(JsonLdError::expansion(error)))
        }
    }

    /// Compacts `document` under this active context (JSON-LD 1.1 API section 4.2) into a document of type `D`,
    /// whose `@context` is this active context's user `@context`.
    ///
    /// A term with no definition in the context compacts to its Fully Qualified Name (clause 5.5.7).
    ///
    /// # Errors
    ///
    /// Returns [`DocumentCompactionError::Compaction`] when the JSON-LD compaction algorithm fails,
    /// [`DocumentCompactionError::NotAnObject`] when the expanded document holds no node object or several, and
    /// [`DocumentCompactionError::Document`] when the compacted value is not a valid `D`.
    pub async fn compact_document<D: LinkedDocument>(&self, document: &ExpandedDocument) -> Result<D, DocumentCompactionError<D::ReadError>> {
        let loader = CoreContextLoader::new(self.bundled(), &OfflineLoader);
        let mut json = document
            .0
            .objects()
            .compact_fragment_full(
                no_vocabulary_mut(),
                self.processed(),
                self.processed(),
                None,
                &loader,
                CompactionOptions::default(),
                &mut (),
            )
            .await
            .map_err(|error| DocumentCompactionError::Compaction(JsonLdError::compaction(error)))?;
        if !json.is_object() {
            return Err(DocumentCompactionError::NotAnObject);
        }
        D::SHAPE.restore(&mut json);
        let mut compacted = D::read(Seal::new(), json).map_err(DocumentCompactionError::Document)?;
        // Clone: the document owns its `@context`, while the active context keeps its own.
        compacted.set_context(Seal::new(), self.source().cloned());
        Ok(compacted)
    }
}

/// Reads a document whose reader takes JSON text: the compacted value is printed for it.
fn printed(json: &Json) -> String {
    json.compact_print().to_string()
}

#[cfg(test)]
mod tests;
