//! The record of a deleted instance a client sends in a temporal evolution (NGSI-LD clauses 4.5.7 and 4.5.8).

use crate::model::{Attribute, AttributeKind, DatasetId, DateTime, Input, InstanceId, Output, evolution::TemporalInstanceMembers};
use bon::bon;

/// The record of a deleted attribute in a temporal evolution a client sends.
///
/// NGSI-LD clauses 4.5.7 and 4.5.8: when a Property or Relationship is deleted, "an instance of the Property is
/// recorded with its value set to the URI `urn:ngsi-ld:null` and the `deletedAt` Temporal Property set". Create or
/// Update Temporal Evolution and Add Attributes take that temporal representation, so a client that replays a history,
/// or a broker that forwards one, sends the deletion as an instance of its own. The payload is the NGSI-LD Null in the
/// shape the kind needs (clause 4.5.0), which the builder writes; the instance has no other value.
///
/// Clause 5.5.4 keeps the Null out of every other part of a request, so a bare Null with no `deletedAt` is still
/// refused.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::model::{
///     AttributeKind,
///     Lossless,
///     evolution::{DeletedInstance, TemporalEntity},
/// };
///
/// let vehicle = TemporalEntity::builder()
///     .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
///     .entity_type("Vehicle")
///     .deleted(
///         "speed",
///         DeletedInstance::builder(AttributeKind::Property, "2018-08-01T12:07:00Z".parse()?)
///             .observed_at("2018-08-01T12:07:00Z".parse()?)
///             .build(),
///     )
///     .build()?;
/// assert_eq!(
///     vehicle.render(Lossless::Concise).to_string(),
///     concat!(
///         r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","#,
///         r#""speed":[{"value":"urn:ngsi-ld:null","#,
///         r#""observedAt":"2018-08-01T12:07:00Z","#,
///         r#""deletedAt":"2018-08-01T12:07:00Z"}]}"#
///     )
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletedInstance {
    /// The kind of the attribute that was deleted, which decides the shape of the NGSI-LD Null.
    kind: AttributeKind,
    /// `datasetId` of the deleted instance.
    dataset_id: Option<DatasetId>,
    /// `observedAt`.
    observed_at: Option<DateTime>,
    /// `deletedAt`.
    deleted_at: DateTime,
    /// `instanceId`.
    instance_id: Option<InstanceId>,
}

#[bon]
impl DeletedInstance {
    /// Starts the builder of the record of a deleted attribute of `kind`, deleted at `deleted_at`. Building cannot
    /// fail.
    #[builder]
    pub const fn new(
        #[builder(start_fn)] kind: AttributeKind,
        #[builder(start_fn)] deleted_at: DateTime,
        #[builder(into)] dataset_id: Option<DatasetId>,
        observed_at: Option<DateTime>,
        #[builder(into)] instance_id: Option<InstanceId>,
    ) -> Self {
        Self {
            kind,
            dataset_id,
            observed_at,
            deleted_at,
            instance_id,
        }
    }
}

impl DeletedInstance {
    /// Splits the record into the attribute that holds the Null and the members that make it a deletion.
    pub(crate) fn into_instance(self) -> (Attribute<Input>, TemporalInstanceMembers) {
        let record = Attribute::<Output>::deleted(self.kind)
            .maybe_dataset_id(self.dataset_id)
            .maybe_observed_at(self.observed_at)
            .build();
        // The record holds no sub-attribute, so converting it can report nothing; the Null is its point.
        let attribute = record.into_input_keeping_null(&mut Vec::new());
        (attribute, TemporalInstanceMembers::new(self.instance_id, Some(self.deleted_at)))
    }
}
