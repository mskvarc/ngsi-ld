//! Merge Entity (clause 6.5.3.4), once per representation of the fragment: normalized or concise, whose fragment
//! names the kind of every attribute, and simplified (`format=simplified`), whose fragment holds bare values the
//! broker merges into the attributes the entity already has (clause 5.6.17.4).

use crate::{
    client::{Answering, ClientError, Entities, Transport, arguments::built},
    http::{
        BatchOutcome,
        operation::{ArgumentError, MergeEntity, MergeSimplifiedEntity},
    },
    model::{Completeness, DateTime, EntityFragment, EntityId, LanguageTagInput, Lossless, SimplifiedFragment, Tenant},
    query::{
        Locality,
        type_selection::{TypeSelection, builders::TypeSelectionBuilder},
    },
};
use bon::bon;

#[bon]
impl<T: Transport, M: Answering> Entities<'_, T, M> {
    /// Merge Entity (clause 6.5.3.4) with a fragment in normalized or concise representation.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the entity does not exist or the call fails.
    #[builder(finish_fn = send)]
    pub async fn merge<C: Completeness>(
        &self,
        #[builder(start_fn)] id: &EntityId,
        #[builder(start_fn)] fragment: &EntityFragment<C>,
        #[builder(into)] entity_type: Option<TypeSelectionBuilder>,
        observed_at: Option<&DateTime>,
        #[builder(default)] form: Lossless,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<BatchOutcome>, ClientError<T::Error>> {
        let entity_type: Option<TypeSelection> = built(entity_type)?;
        self.route
            .send_with_locality(
                locality,
                &MergeEntity::builder()
                    .id(id)
                    .fragment(fragment)
                    .maybe_entity_type(entity_type.as_ref())
                    .maybe_observed_at(observed_at)
                    .form(form)
                    .build(),
                tenant,
            )
            .await
    }

    /// Merge Entity (clause 6.5.3.4) with a fragment in simplified representation (`format=simplified`): every
    /// attribute keeps the kind the entity gives it (clause 5.6.17.4). `lang` is the language of the strings the
    /// fragment holds: a string merged into a `LanguageProperty` replaces the text of that language only. It is
    /// given either as text, such as `"fr"`, which the call parses, or as a [`LangTagBuf`](langtag::LangTagBuf)
    /// already parsed.
    ///
    /// ```
    /// # use bytes::Bytes;
    /// # use http::{Request, Response, StatusCode};
    /// # use std::convert::Infallible;
    /// # #[derive(Clone)]
    /// # struct Offline;
    /// # impl ngsi_ld::client::Transport for Offline {
    /// #     type Error = Infallible;
    /// #     async fn send(&self, _: Request<Bytes>) -> Result<Response<Bytes>, Infallible> {
    /// #         let mut response = Response::new(Bytes::new());
    /// #         *response.status_mut() = StatusCode::NO_CONTENT;
    /// #         Ok(response)
    /// #     }
    /// # }
    /// use iri_rs::iri;
    /// use langtag::LangTagBuf;
    /// use ngsi_ld::{
    ///     client::Client,
    ///     http::BatchOutcome,
    ///     model::{EntityId, SimplifiedFragment},
    /// };
    /// use url::Url;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> anyhow::Result<()> {
    /// let client = Client::builder()
    ///     .base_url(Url::parse("http://localhost:9090/ngsi-ld/v1/")?)
    ///     .transport(Offline)
    ///     .build()?;
    /// let id = EntityId::from(iri!("urn:ngsi-ld:Building:store001"));
    /// let fragment = SimplifiedFragment::builder()
    ///     .value("name", "Grand-Place")
    ///     .build()?;
    ///
    /// // The language as text, parsed by the call, which refuses a non-tag before any request.
    /// let outcome = client
    ///     .entities()
    ///     .merge_simplified(&id, &fragment)
    ///     .lang("fr")
    ///     .send()
    ///     .await?;
    /// assert_eq!(outcome, BatchOutcome::Done);
    ///
    /// // Or as a language tag parsed beforehand.
    /// let french: LangTagBuf = "fr".parse()?;
    /// let outcome = client
    ///     .entities()
    ///     .merge_simplified(&id, &fragment)
    ///     .lang(&french)
    ///     .send()
    ///     .await?;
    /// assert_eq!(outcome, BatchOutcome::Done);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when `lang` is text that is not a language tag (IETF RFC 5646), the entity does not
    /// exist, or the call fails.
    #[builder(finish_fn = send)]
    pub async fn merge_simplified(
        &self,
        #[builder(start_fn)] id: &EntityId,
        #[builder(start_fn)] fragment: &SimplifiedFragment,
        #[builder(into)] entity_type: Option<TypeSelectionBuilder>,
        observed_at: Option<&DateTime>,
        #[builder(into)] lang: Option<LanguageTagInput<'_>>,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<BatchOutcome>, ClientError<T::Error>> {
        let entity_type: Option<TypeSelection> = built(entity_type)?;
        let lang = lang
            .map(LanguageTagInput::resolve)
            .transpose()
            .map_err(|source| ClientError::Request(ArgumentError::LanguageTag { source }.into()))?;
        self.route
            .send_with_locality(
                locality,
                &MergeSimplifiedEntity::builder()
                    .id(id)
                    .fragment(fragment)
                    .maybe_entity_type(entity_type.as_ref())
                    .maybe_observed_at(observed_at)
                    .maybe_lang(lang.as_deref())
                    .build(),
                tenant,
            )
            .await
    }
}
