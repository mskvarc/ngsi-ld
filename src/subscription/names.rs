//! The names a subscription holds, query-language members included (NGSI-LD clauses 5.2.12, 5.2.14 and 5.5.7):
//! the entity types it selects, the attributes it watches, expands, keeps as JSON or notifies, and every attribute
//! its `q`, `csf`, `geoQ`, `pick` and `omit` name.

use crate::{
    model::{
        DocumentError,
        name::mapping::{MapNames, NameMapping, member},
    },
    subscription::{
        EntitySubscriptionChanges,
        EntitySubscriptionMembers,
        NotificationMember,
        NotificationParams,
        RegistrationSubscriptionChanges,
        RegistrationSubscriptionMembers,
        Subscription,
        SubscriptionDirection,
        SubscriptionFragment,
        SubscriptionKind,
        SubscriptionMember,
        kind::sealed::KindMembers,
        member::SubscriptionMandatoryMember,
    },
};

impl<D: SubscriptionDirection, K: SubscriptionKind> MapNames for Subscription<D, K>
where
    K::Members: MapNames,
{
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            entities: member(self.entities, SubscriptionMember::Entities.into(), mapping)?,
            watched_attributes: member(self.watched_attributes, SubscriptionMember::WatchedAttributes.into(), mapping)?,
            q: member(self.q, SubscriptionMember::Q.into(), mapping)?,
            geo_q: member(self.geo_q, SubscriptionMember::GeoQ.into(), mapping)?,
            kind_members: self.kind_members.map_names(mapping)?,
            expand_values: member(self.expand_values, SubscriptionMember::ExpandValues.into(), mapping)?,
            json_keys: member(self.json_keys, SubscriptionMember::JsonKeys.into(), mapping)?,
            notification: member(self.notification, SubscriptionMandatoryMember::Notification.into(), mapping)?,
            ..self
        })
    }
}

impl<K: SubscriptionKind> MapNames for SubscriptionFragment<K>
where
    <K::Members as KindMembers>::Changes: MapNames,
{
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            entities: member(self.entities, SubscriptionMember::Entities.into(), mapping)?,
            watched_attributes: member(self.watched_attributes, SubscriptionMember::WatchedAttributes.into(), mapping)?,
            q: member(self.q, SubscriptionMember::Q.into(), mapping)?,
            geo_q: member(self.geo_q, SubscriptionMember::GeoQ.into(), mapping)?,
            kind_members: self.kind_members.map_names(mapping)?,
            expand_values: member(self.expand_values, SubscriptionMember::ExpandValues.into(), mapping)?,
            json_keys: member(self.json_keys, SubscriptionMember::JsonKeys.into(), mapping)?,
            notification: member(self.notification, SubscriptionMandatoryMember::Notification.into(), mapping)?,
            ..self
        })
    }
}

/// `attributes`, `pick` and `omit` name what each notification carries; the endpoint names none.
impl<D: SubscriptionDirection> MapNames for NotificationParams<D> {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            attributes: member(self.attributes, NotificationMember::Attributes.into(), mapping)?,
            pick: member(self.pick, NotificationMember::Pick.into(), mapping)?,
            omit: member(self.omit, NotificationMember::Omit.into(), mapping)?,
            ..self
        })
    }
}

impl MapNames for EntitySubscriptionMembers {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            csf: member(self.csf, SubscriptionMember::Csf.into(), mapping)?,
        })
    }
}

impl MapNames for EntitySubscriptionChanges {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            csf: member(self.csf, SubscriptionMember::Csf.into(), mapping)?,
        })
    }
}

/// A `temporalQ` names a system temporal property, never an attribute (clause 4.11).
impl MapNames for RegistrationSubscriptionMembers {
    fn map_names<M: NameMapping>(self, _mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(self)
    }
}

/// A `temporalQ` names a system temporal property, never an attribute (clause 4.11).
impl MapNames for RegistrationSubscriptionChanges {
    fn map_names<M: NameMapping>(self, _mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(self)
    }
}

carries_context!(impl<D: SubscriptionDirection, K: SubscriptionKind> for Subscription<D, K> => context);

carries_context!(
    /// A fragment carries no `@context`: its request delivers the endpoint's (clause 6.3.5).
    impl<K: SubscriptionKind> for SubscriptionFragment<K>
);
