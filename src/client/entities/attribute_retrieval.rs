//! Retrieve Attribute and Retrieve Attribute Value (ETSI TS 104 176 clauses 7.5.3.1 and 7.6.3.1).

use crate::{
    client::{Answering, ClientError, Entities, Transport, arguments::built},
    http::{
        Retrieved,
        operation::{EntityMapRequest, InlineJoin, RetrieveAttribute, RetrieveAttributeValue, SystemAttributes},
    },
    model::{Attribute, AttributeDocument, AttributeName, DatasetId, EntityId, Output, RawJson, Tenant},
    query::{
        Locality,
        language_filter::{LanguageFilter, builders::LanguageFilterBuilder},
        type_selection::{TypeSelection, builders::TypeSelectionBuilder},
    },
};
use bon::bon;
use std::num::NonZeroU32;

#[bon]
impl<T: Transport, M: Answering> Entities<'_, T, M> {
    /// Retrieve Attribute (ETSI TS 104 176 clause 7.5.3.1): one instance of one attribute of the entity, the one
    /// `dataset_id` names or else the default instance, in normalized representation.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the entity or the instance does not exist, an argument is invalid, or the call
    /// fails.
    #[builder(finish_fn = send)]
    pub async fn retrieve_attribute(
        &self,
        #[builder(start_fn)] id: &EntityId,
        #[builder(start_fn)] name: &AttributeName,
        #[builder(into)] entity_type: Option<TypeSelectionBuilder>,
        dataset_id: Option<&DatasetId>,
        #[builder(into)] lang: Option<LanguageFilterBuilder>,
        join: Option<InlineJoin>,
        join_level: Option<NonZeroU32>,
        #[builder(default)] contained_by: Vec<EntityId>,
        entity_map: Option<&EntityMapRequest>,
        #[builder(default)] system_attributes: SystemAttributes,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<Retrieved<AttributeDocument<Attribute<Output>>>>, ClientError<T::Error>> {
        let entity_type: Option<TypeSelection> = built(entity_type)?;
        let lang: Option<LanguageFilter> = built(lang)?;
        let retrieve = RetrieveAttribute {
            id,
            name,
            entity_type: entity_type.as_ref(),
            dataset_id,
            lang: lang.as_ref(),
            join,
            join_level,
            contained_by: &contained_by,
            entity_map,
            system_attributes,
        };
        self.route.send_with_locality(locality, &retrieve, tenant).await
    }

    /// Retrieve Attribute Value (ETSI TS 104 176 clause 7.6.3.1): the value of one instance of one attribute of the
    /// entity, the one `dataset_id` names or else the default instance, as the broker wrote it.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the entity or the instance does not exist, an argument is invalid, or the call
    /// fails.
    #[builder(finish_fn = send)]
    pub async fn retrieve_attribute_value(
        &self,
        #[builder(start_fn)] id: &EntityId,
        #[builder(start_fn)] name: &AttributeName,
        #[builder(into)] entity_type: Option<TypeSelectionBuilder>,
        dataset_id: Option<&DatasetId>,
        #[builder(into)] lang: Option<LanguageFilterBuilder>,
        entity_map: Option<&EntityMapRequest>,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<Retrieved<RawJson>>, ClientError<T::Error>> {
        let entity_type: Option<TypeSelection> = built(entity_type)?;
        let lang: Option<LanguageFilter> = built(lang)?;
        let retrieve = RetrieveAttributeValue {
            id,
            name,
            entity_type: entity_type.as_ref(),
            dataset_id,
            lang: lang.as_ref(),
            entity_map,
        };
        self.route.send_with_locality(locality, &retrieve, tenant).await
    }
}
