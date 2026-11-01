//! Set Attribute Value (ETSI TS 104 176 clause 7.6.3.2).

use crate::{
    client::{Answering, ClientError, Entities, Transport, arguments::built},
    http::{
        BatchOutcome,
        operation::{SetAttributeValue, ValueOnly},
    },
    model::{AttributeName, DatasetId, DateTime, EntityId, Tenant},
    query::{
        Locality,
        type_selection::{TypeSelection, builders::TypeSelectionBuilder},
    },
};
use bon::bon;

#[bon]
impl<T: Transport, M: Answering> Entities<'_, T, M> {
    /// Set Attribute Value (ETSI TS 104 176 clause 7.6.3.2): sets the value of one instance of one attribute of the
    /// entity, the one `dataset_id` names or else the default instance, keeping its kind and its other members.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the entity or the instance does not exist, the value holds the NGSI-LD Null, an
    /// argument is invalid, or the call fails.
    #[builder(finish_fn = send)]
    pub async fn set_attribute_value(
        &self,
        #[builder(start_fn)] id: &EntityId,
        #[builder(start_fn)] name: &AttributeName,
        #[builder(start_fn, into)] value: ValueOnly<'_>,
        #[builder(into)] entity_type: Option<TypeSelectionBuilder>,
        dataset_id: Option<&DatasetId>,
        observed_at: Option<&DateTime>,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<BatchOutcome>, ClientError<T::Error>> {
        let entity_type: Option<TypeSelection> = built(entity_type)?;
        let set = SetAttributeValue {
            id,
            name,
            value,
            entity_type: entity_type.as_ref(),
            dataset_id,
            observed_at,
        };
        self.route.send_with_locality(locality, &set, tenant).await
    }
}
