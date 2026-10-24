//! Resource paths of the NGSI-LD API (NGSI-LD clause 6.2, Table 6.2-1), with each variable percent-encoded as a
//! path segment.

use crate::{
    model::{AttributeName, ContextId, EntityId, EntityTypeName, InstanceId, Uri},
    registration::{EntityMapId, RegistrationId},
    snapshot::SnapshotId,
    subscription::SubscriptionId,
};
use percent_encoding::{AsciiSet, CONTROLS, percent_decode_str, utf8_percent_encode};
use std::str::FromStr;

/// The characters a path segment cannot carry verbatim: everything outside `pchar` (IETF RFC 3986 section 3.3).
/// `:` and `@` are `pchar`, so a URN id keeps its colons; `/`, `?` and `#` would end the segment, and `%` would
/// start an escape.
const PATH_SEGMENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'/')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}');

/// The batch sub-resources of `entityOperations` (Table 6.2-1); the token is the path segment the specification
/// gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display)]
#[strum(serialize_all = "lowercase")]
pub(crate) enum BatchResource {
    /// `create` (clause 6.14).
    Create,
    /// `upsert` (clause 6.15).
    Upsert,
    /// `update` (clause 6.16).
    Update,
    /// `delete` (clause 6.17).
    Delete,
    /// `merge` (clause 6.31).
    Merge,
}

/// The two collections of subscriptions (Table 6.2-1): context information subscriptions (clause 6.10) and
/// Context Source Registration Subscriptions (clause 6.12), which share the Subscription data type and their
/// operations. The token is the path segment the specification gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display)]
pub enum SubscriptionCollection {
    /// `subscriptions`.
    #[strum(serialize = "subscriptions")]
    Subscriptions,
    /// `csourceSubscriptions`.
    #[strum(serialize = "csourceSubscriptions")]
    CsourceSubscriptions,
}

/// A resource path below the API root, already percent-encoded, without a leading `/`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub(crate) struct ResourcePath(String);

impl ResourcePath {
    /// `entities` (clause 6.4).
    pub(crate) fn entities() -> Self {
        Self("entities".to_owned())
    }

    /// `entities/{entityId}` (clause 6.5).
    pub(crate) fn entity(id: &EntityId) -> Self {
        Self::entities().segment(id.as_str())
    }

    /// `entities/{entityId}/attrs` (clause 6.6).
    pub(crate) fn attributes(id: &EntityId) -> Self {
        Self::entity(id).segment("attrs")
    }

    /// `entities/{entityId}/attrs/{attrId}` (clause 6.7).
    pub(crate) fn attribute(id: &EntityId, name: &AttributeName) -> Self {
        Self::attributes(id).segment(name.as_ref())
    }

    /// `entityOperations/query` (clause 6.23).
    pub(crate) fn query_entities() -> Self {
        Self("entityOperations/query".to_owned())
    }

    /// `entityOperations/{create,upsert,update,delete,merge}` (clauses 6.14 to 6.17 and 6.31).
    pub(crate) fn batch(operation: BatchResource) -> Self {
        Self(format!("entityOperations/{operation}"))
    }

    /// `subscriptions` (clause 6.10) or `csourceSubscriptions` (clause 6.12).
    pub(crate) fn subscriptions(collection: SubscriptionCollection) -> Self {
        Self(collection.to_string())
    }

    /// `subscriptions/{subscriptionId}` (clause 6.11) or `csourceSubscriptions/{subscriptionId}` (clause 6.13).
    pub(crate) fn subscription(collection: SubscriptionCollection, id: &SubscriptionId) -> Self {
        Self::subscriptions(collection).segment(id.as_str())
    }

    /// `csourceRegistrations` (clause 6.8).
    pub(crate) fn registrations() -> Self {
        Self("csourceRegistrations".to_owned())
    }

    /// `csourceRegistrations/{registrationId}` (clause 6.9).
    pub(crate) fn registration(id: &RegistrationId) -> Self {
        Self::registrations().segment(id.as_str())
    }

    /// `temporal/entities` (clause 6.18).
    pub(crate) fn temporal_entities() -> Self {
        Self("temporal/entities".to_owned())
    }

    /// `temporal/entities/{entityId}` (clause 6.19).
    pub(crate) fn temporal_entity(id: &EntityId) -> Self {
        Self::temporal_entities().segment(id.as_str())
    }

    /// `temporal/entities/{entityId}/attrs` (clause 6.20).
    pub(crate) fn temporal_attributes(id: &EntityId) -> Self {
        Self::temporal_entity(id).segment("attrs")
    }

    /// `temporal/entities/{entityId}/attrs/{attrId}` (clause 6.21).
    pub(crate) fn temporal_attribute(id: &EntityId, name: &AttributeName) -> Self {
        Self::temporal_attributes(id).segment(name.as_ref())
    }

    /// `temporal/entities/{entityId}/attrs/{attrId}/{instanceId}` (clause 6.22).
    pub(crate) fn temporal_instance(id: &EntityId, name: &AttributeName, instance: &InstanceId) -> Self {
        let instance: &Uri = instance.as_ref();
        Self::temporal_attribute(id, name).segment(instance.as_str())
    }

    /// `temporal/entityOperations/query` (clause 6.24).
    pub(crate) fn query_temporal() -> Self {
        Self("temporal/entityOperations/query".to_owned())
    }

    /// `types` (clause 6.25).
    pub(crate) fn types() -> Self {
        Self("types".to_owned())
    }

    /// `types/{type}` (clause 6.26).
    pub(crate) fn entity_type(name: &EntityTypeName) -> Self {
        Self::types().segment(name.as_ref())
    }

    /// `attributes` (clause 6.27).
    pub(crate) fn available_attributes() -> Self {
        Self("attributes".to_owned())
    }

    /// `attributes/{attrId}` (clause 6.28).
    pub(crate) fn available_attribute(name: &AttributeName) -> Self {
        Self::available_attributes().segment(name.as_ref())
    }

    /// `jsonldContexts` (clause 6.29).
    pub(crate) fn contexts() -> Self {
        Self("jsonldContexts".to_owned())
    }

    /// `jsonldContexts/{contextId}` (clause 6.30).
    pub(crate) fn stored_context(id: &ContextId) -> Self {
        Self::contexts().segment(id.as_str())
    }

    /// `entityMaps` (clause 6.34).
    pub(crate) fn entity_maps() -> Self {
        Self("entityMaps".to_owned())
    }

    /// `entityMaps/{entityMapId}` (clause 6.32).
    pub(crate) fn entity_map(id: &EntityMapId) -> Self {
        Self::entity_maps().segment(id.as_str())
    }

    /// `temporal/entityMaps` (clause 6.35).
    pub(crate) fn temporal_entity_maps() -> Self {
        Self("temporal/entityMaps".to_owned())
    }

    /// `info/sourceIdentity` (clause 6.33).
    pub(crate) fn source_identity() -> Self {
        Self("info/sourceIdentity".to_owned())
    }

    /// `snapshots` (clause 6.36).
    pub(crate) fn snapshots() -> Self {
        Self("snapshots".to_owned())
    }

    /// `snapshots/{snapshotId}` (clause 6.37).
    pub(crate) fn snapshot(id: &SnapshotId) -> Self {
        Self::snapshots().segment(id.as_str())
    }

    /// `snapshots/{snapshotId}/clone` (clause 6.38).
    pub(crate) fn snapshot_clone(id: &SnapshotId) -> Self {
        let mut path = Self::snapshot(id);
        path.0.push_str("/clone");
        path
    }

    /// Appends one segment, percent-encoding what a segment cannot carry.
    fn segment(mut self, segment: &str) -> Self {
        self.0.push('/');
        self.0.extend(utf8_percent_encode(segment, PATH_SEGMENT));
        self
    }
}

/// The identifier a resource location names in its last path segment, percent-decoded: Table 6.2-1 makes that
/// segment the id of the resource (`/subscriptions/{subscriptionId}`, `/entityMaps/{entityMapId}`, ...). `None` when
/// the segment is not an identifier.
pub(crate) fn id_from_location<I: FromStr>(location: &str) -> Option<I> {
    let path = location.split(['?', '#']).next()?;
    let segment = path.rsplit('/').next()?;
    percent_decode_str(segment).decode_utf8().ok()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use crate::{
        http::path::{BatchResource, ResourcePath, SubscriptionCollection},
        model::{AttributeName, EntityId, InstanceId},
    };
    use iri_rs::iri;
    use pretty_assertions::assert_eq;

    #[test]
    fn a_urn_entity_id_keeps_its_colons_in_the_path() {
        assert_eq!(
            ResourcePath::entity(&iri!("urn:ngsi-ld:Vehicle:A4567").into()).to_string(),
            "entities/urn:ngsi-ld:Vehicle:A4567"
        );
    }

    #[test]
    fn slash_and_hash_in_an_entity_id_are_percent_encoded() {
        let id: EntityId = "https://example.org/vehicles/A4567#front".parse().unwrap();
        assert_eq!(
            ResourcePath::entity(&id).to_string(),
            "entities/https:%2F%2Fexample.org%2Fvehicles%2FA4567%23front"
        );
    }

    #[test]
    fn an_attribute_named_by_an_iri_is_one_encoded_segment() {
        let name: AttributeName = "https://example.org/speed".parse().unwrap();
        assert_eq!(
            ResourcePath::attribute(&iri!("urn:ngsi-ld:Vehicle:A4567").into(), &name).to_string(),
            "entities/urn:ngsi-ld:Vehicle:A4567/attrs/https:%2F%2Fexample.org%2Fspeed"
        );
    }

    #[test]
    fn the_batch_resources_are_below_entity_operations() {
        let paths = [
            BatchResource::Create,
            BatchResource::Upsert,
            BatchResource::Update,
            BatchResource::Delete,
            BatchResource::Merge,
        ]
        .map(|operation| ResourcePath::batch(operation).to_string());
        assert_eq!(
            paths,
            [
                "entityOperations/create",
                "entityOperations/upsert",
                "entityOperations/update",
                "entityOperations/delete",
                "entityOperations/merge"
            ]
        );
    }

    #[test]
    fn the_temporal_resources_encode_each_variable_as_one_segment() {
        let id: EntityId = "urn:ngsi-ld:Vehicle:B9211".parse().unwrap();
        let name: AttributeName = "https://example.org/speed".parse().unwrap();
        let instance: InstanceId = "urn:ngsi-ld:attribute:instance:1".parse().unwrap();
        assert_eq!(ResourcePath::temporal_entities().to_string(), "temporal/entities");
        assert_eq!(
            ResourcePath::temporal_instance(&id, &name, &instance).to_string(),
            "temporal/entities/urn:ngsi-ld:Vehicle:B9211/attrs/https:%2F%2Fexample.org%2Fspeed/urn:ngsi-ld:attribute:instance:1"
        );
        assert_eq!(ResourcePath::query_temporal().to_string(), "temporal/entityOperations/query");
    }

    #[test]
    fn the_registry_resources_encode_each_variable_as_one_segment() {
        let subscription = "https://example.org/s/1".parse().unwrap();
        let registration = "urn:ngsi-ld:ContextSourceRegistration:csr1".parse().unwrap();
        assert_eq!(
            ResourcePath::subscription(SubscriptionCollection::Subscriptions, &subscription).to_string(),
            "subscriptions/https:%2F%2Fexample.org%2Fs%2F1"
        );
        assert_eq!(
            ResourcePath::subscription(SubscriptionCollection::CsourceSubscriptions, &subscription).to_string(),
            "csourceSubscriptions/https:%2F%2Fexample.org%2Fs%2F1"
        );
        assert_eq!(
            ResourcePath::registration(&registration).to_string(),
            "csourceRegistrations/urn:ngsi-ld:ContextSourceRegistration:csr1"
        );
    }

    #[test]
    fn the_entity_map_and_source_identity_resources_follow_table_6_2_1() {
        let map = "https://example.org/maps/1#a".parse().unwrap();
        assert_eq!(ResourcePath::entity_map(&map).to_string(), "entityMaps/https:%2F%2Fexample.org%2Fmaps%2F1%23a");
        assert_eq!(ResourcePath::entity_maps().to_string(), "entityMaps");
        assert_eq!(ResourcePath::temporal_entity_maps().to_string(), "temporal/entityMaps");
        assert_eq!(ResourcePath::source_identity().to_string(), "info/sourceIdentity");
    }

    #[test]
    fn non_ascii_characters_are_encoded_as_utf8() {
        let id: EntityId = "urn:ngsi-ld:Building:München".parse().unwrap();
        assert_eq!(ResourcePath::entity(&id).to_string(), "entities/urn:ngsi-ld:Building:M%C3%BCnchen");
    }
}
