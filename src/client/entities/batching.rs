//! The batch operations (clauses 6.14 to 6.17 and 6.31): several entities in one request, each entity reported
//! on its own.

use crate::{
    client::{Answering, ClientError, Entities, Transport},
    http::{
        BatchCreateOutcome,
        BatchOutcome,
        UpsertOutcome,
        operation::{BatchCreate, BatchDelete, BatchMerge, BatchUpdate, BatchUpsert, Overwrite, UpsertMode},
    },
    model::{Complete, Entity, EntityFragment, EntityId, Input, Lossless, Tenant},
    query::Locality,
};
use bon::bon;

#[bon]
impl<T: Transport, M: Answering> Entities<'_, T, M> {
    /// Batch Entity Creation (clause 6.14.3.1): creates every entity; of several instances of one entity, the
    /// first creates it and the later ones are reported as `AlreadyExists` (clause 5.5.11.1).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the batch is empty (refused before sending), the broker rejects the request as
    /// a whole, or the call fails. Entities that could not be created are reported in
    /// [`BatchCreateOutcome::Partial`].
    #[builder(finish_fn = send)]
    pub async fn batch_create(
        &self,
        #[builder(start_fn)] entities: &[Entity<Input>],
        #[builder(default)] form: Lossless,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<BatchCreateOutcome>, ClientError<T::Error>> {
        self.route
            .send_with_locality(locality, &BatchCreate::builder().entities(entities).form(form).build(), tenant)
            .await
    }

    /// Batch Entity Upsert (clause 6.15.3.1): creates every entity that does not exist, and replaces, or with
    /// [`UpsertMode::Update`] updates, every one that does.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the batch is empty (refused before sending), the broker rejects the request as
    /// a whole, or the call fails. Entities that could not be created or updated are reported in
    /// [`UpsertOutcome::Partial`].
    #[builder(finish_fn = send)]
    pub async fn batch_upsert(
        &self,
        #[builder(start_fn)] entities: &[Entity<Input>],
        #[builder(default)] mode: UpsertMode,
        #[builder(default)] form: Lossless,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<UpsertOutcome>, ClientError<T::Error>> {
        self.route
            .send_with_locality(locality, &BatchUpsert::builder().entities(entities).mode(mode).form(form).build(), tenant)
            .await
    }

    /// Batch Entity Update (clause 6.16.3.1): changes the attributes of every entity a fragment names, keeping
    /// existing attributes with [`Overwrite::Keep`].
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the batch is empty (refused before sending), the broker rejects the request as
    /// a whole, or the call fails. Entities that could not be updated are reported in [`BatchOutcome::Partial`].
    #[builder(finish_fn = send)]
    pub async fn batch_update(
        &self,
        #[builder(start_fn)] fragments: &[EntityFragment<Complete>],
        #[builder(default)] overwrite: Overwrite,
        #[builder(default)] form: Lossless,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<BatchOutcome>, ClientError<T::Error>> {
        self.route
            .send_with_locality(
                locality,
                &BatchUpdate::builder().fragments(fragments).overwrite(overwrite).form(form).build(),
                tenant,
            )
            .await
    }

    /// Batch Entity Merge (clause 6.31.3.1): merges every fragment into the entity it names.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the batch is empty (refused before sending), the broker rejects the request as
    /// a whole, or the call fails. Entities that could not be merged are reported in [`BatchOutcome::Partial`].
    #[builder(finish_fn = send)]
    pub async fn batch_merge(
        &self,
        #[builder(start_fn)] fragments: &[EntityFragment<Complete>],
        #[builder(default)] form: Lossless,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<BatchOutcome>, ClientError<T::Error>> {
        self.route
            .send_with_locality(locality, &BatchMerge::builder().fragments(fragments).form(form).build(), tenant)
            .await
    }

    /// Batch Entity Delete (clause 6.17.3.1): deletes every entity named; of several occurrences of one id, the
    /// later ones are reported as not found (clause 5.5.11.4).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the batch is empty (refused before sending), the broker rejects the request as
    /// a whole, or the call fails. Entities that could not be deleted are reported in [`BatchOutcome::Partial`].
    #[builder(finish_fn = send)]
    pub async fn batch_delete(
        &self,
        #[builder(start_fn)] ids: &[EntityId],
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<BatchOutcome>, ClientError<T::Error>> {
        self.route.send_with_locality(locality, &BatchDelete { ids }, tenant).await
    }
}
