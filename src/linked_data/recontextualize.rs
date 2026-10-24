//! Re-contextualisation: a document written under one `@context`, rewritten under another, every name translated
//! through its Fully Qualified Name (NGSI-LD clause 5.5.7).

use crate::{
    linked_data::{ActiveContext, TranslationError},
    model::{AttributeName, AttributesError, DocumentError, EntityTypeName, Term, VocabTerm, name::mapping::NameMapping},
    query::type_selection::SelectableTypeError,
    seal::Seal,
};

/// Why a document could not be re-contextualised; the [`DocumentError`] holding it names the member. A
/// translation error is boxed, being much larger than the others.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum NameMappingError {
    /// An entity type did not translate.
    #[error("an entity type does not translate")]
    EntityType(#[source] Box<TranslationError<EntityTypeName>>),
    /// An attribute name did not translate.
    #[error("an attribute name does not translate")]
    Attribute(#[source] Box<TranslationError<AttributeName>>),
    /// A bare term of a query language did not translate.
    #[error("a query-language term does not translate")]
    Term(#[source] Box<TranslationError<Term>>),
    /// A vocabulary term did not translate.
    #[error("a vocabulary term does not translate")]
    Vocab(#[source] Box<TranslationError<VocabTerm>>),
    /// The translated attribute cannot stand under its new name: two names of the source context translate to one
    /// term, or a name the core context reserves for a `GeoProperty` now names another kind.
    #[error("the translated attribute cannot stand under its name")]
    Attributes(#[from] AttributesError),
    /// The translated entity type holds an operator of the type selection language.
    #[error("the translated entity type cannot stand in a type selection")]
    Selection(#[from] SelectableTypeError),
}

/// A document [`ActiveContext::recontextualize`] rewrites: every document of the crate and every query-language
/// value a request carries, fragments included.
///
/// The trait is sealed.
pub trait Recontextualize: sealed::Sealed {}

impl<D: sealed::Sealed> Recontextualize for D {}

/// The hooks every [`Recontextualize`] provides.
mod sealed {
    use crate::model::name::mapping::{CarriesContext, MapNames};

    /// A value whose names are mapped and whose `@context` is replaced.
    pub trait Sealed: MapNames + CarriesContext {}

    impl<D: MapNames + CarriesContext> Sealed for D {}
}

/// The translation of every name from one active context into another.
struct Translation<'a> {
    /// The context the document is written under.
    from: &'a ActiveContext,
    /// The context it is rewritten under.
    to: &'a ActiveContext,
}

impl NameMapping for Translation<'_> {
    type Error = NameMappingError;

    fn entity_type(&self, name: &EntityTypeName) -> Result<EntityTypeName, Self::Error> {
        self.from
            .translate(name, self.to)
            .map_err(|error| NameMappingError::EntityType(Box::new(error)))
    }

    fn attribute(&self, name: &AttributeName) -> Result<AttributeName, Self::Error> {
        self.from.translate(name, self.to).map_err(|error| NameMappingError::Attribute(Box::new(error)))
    }

    fn term(&self, name: &Term) -> Result<Term, Self::Error> {
        self.from.translate(name, self.to).map_err(|error| NameMappingError::Term(Box::new(error)))
    }

    fn vocab(&self, name: &VocabTerm) -> Result<VocabTerm, Self::Error> {
        self.from.translate(name, self.to).map_err(|error| NameMappingError::Vocab(Box::new(error)))
    }
}

impl ActiveContext {
    /// Rewrites `document`, written under `from`, under this active context: every name is translated through
    /// its Fully Qualified Name (clause 5.5.7), and the document's `@context`, when it has one, becomes this
    /// context's user `@context`.
    ///
    /// Unlike [`compact_document`](Self::compact_document), re-contextualisation reaches into the query-language
    /// members: the attributes a `q`, `csf`, `pick`, `omit` or ordering names, and the entity types a type selection
    /// names, are translated like every other name. A query-language term is a bare term (clause 4.9), so a name the
    /// target context has no term for cannot be written in one and is refused, where a member that holds an IRI takes
    /// the Fully Qualified Name.
    ///
    /// ```
    /// use ngsi_ld::{
    ///     linked_data::ContextProcessor,
    ///     model::{Context, ContextEntry, InlineContext},
    ///     query::q::Query,
    /// };
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> anyhow::Result<()> {
    /// let a = Context::Single(ContextEntry::Inline(
    ///     "{\"speed\": \"https://example.org/speed\"}".parse::<InlineContext>()?,
    /// ));
    /// let b = Context::Single(ContextEntry::Inline(
    ///     "{\"velocity\": \"https://example.org/speed\"}".parse::<InlineContext>()?,
    /// ));
    /// let processor = ContextProcessor::try_new()?;
    /// let a = processor.process_offline(&a).await?;
    /// let b = processor.process_offline(&b).await?;
    /// let q: Query = "speed>50".parse()?;
    /// assert_eq!(b.recontextualize(q, &a)?.to_string(), "velocity>50");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns a [`DocumentError`] naming the member at fault, holding [`NameMappingError`] for a name that does
    /// not expand under `from` or does not compact under this context, and for a translated name that cannot
    /// stand where the original stood.
    pub fn recontextualize<D: Recontextualize>(&self, document: D, from: &Self) -> Result<D, DocumentError<NameMappingError>> {
        let mut document = document.map_names(&Translation { from, to: self })?;
        // Clone: the document owns its `@context`, while the active context keeps its own.
        document.set_context(Seal::new(), self.source().cloned());
        Ok(document)
    }
}

#[cfg(test)]
mod tests;
