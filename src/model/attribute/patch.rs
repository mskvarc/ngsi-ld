//! The deletions an attribute fragment makes inside the attribute it changes (NGSI-LD clauses 5.5.8 and 5.5.12): a
//! partial update patch or a merge patch deletes every member of the target whose value in the fragment is the
//! NGSI-LD Null, an optional member or a sub-attribute.

use crate::model::{
    Attribute,
    AttributeKind,
    AttributeNameDraft,
    BuildError,
    BuildProblem,
    Change,
    Deletion,
    MemberChange,
    Patch,
    attribute::pending::{insert, resolve_name},
};

/// An optional member of an attribute a fragment may delete with the NGSI-LD Null (clause 5.5.8): every member
/// besides the payload, whose Null deletes the attribute itself (clauses 4.5.2.2, 4.5.3.2 and their counterparts),
/// and besides `datasetId`, which clause 5.5.8 says "cannot be deleted by setting it to the value
/// "urn:ngsi-ld:null"". The variants are written as the specification spells the members.
///
/// The enum is `#[non_exhaustive]`: a later version of the specification may give attributes more optional members.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr, strum::EnumIter)]
#[non_exhaustive]
pub enum PatchMember {
    /// `observedAt` (clause 4.8), of every kind.
    #[strum(serialize = "observedAt")]
    ObservedAt,
    /// `expiresAt` (clause 4.22), of every kind.
    #[strum(serialize = "expiresAt")]
    ExpiresAt,
    /// `unitCode` (clause 4.5.2.2), of a Property, a `GeoProperty` and a `ListProperty`.
    #[strum(serialize = "unitCode")]
    UnitCode,
    /// `valueType` (clause 4.5.2.2), of every Property kind.
    #[strum(serialize = "valueType")]
    ValueType,
    /// `objectType` (clause 4.5.3.2), of a Relationship and a `ListRelationship`.
    #[strum(serialize = "objectType")]
    ObjectType,
}

/// The attribute kind has no such member: a Relationship has no `unitCode` or `valueType`, and only the
/// relationship kinds have an `objectType` (clauses 4.5.2.2, 4.5.3.2 and their counterparts).
///
/// The struct is `#[non_exhaustive]`: its fields are readable, and only the crate builds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[error("a {kind} has no `{member}` member")]
#[non_exhaustive]
pub struct MemberOfOtherKind {
    /// The member.
    pub member: PatchMember,
    /// The kind of the attribute.
    pub kind: AttributeKind,
}

impl Attribute<Patch> {
    /// The same fragment, deleting `member` of the target attribute: the member is written as the NGSI-LD Null
    /// (clause 5.5.8), replacing any value the fragment set for it.
    ///
    /// ```
    /// use ngsi_ld::model::{Attribute, Lossless, PatchMember, Property};
    ///
    /// let level = Attribute::from(Property::builder(4).build()?)
    ///     .into_patch()
    ///     .delete_member(PatchMember::ObservedAt)?;
    /// assert_eq!(
    ///     level.render(Lossless::Normalized).to_string(),
    ///     r#"{"type":"Property","value":4,"observedAt":"urn:ngsi-ld:null"}"#
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`MemberOfOtherKind`] when the attribute's kind has no such member.
    pub fn delete_member(mut self, member: PatchMember) -> Result<Self, MemberOfOtherKind> {
        let kind = self.kind();
        let refused = MemberOfOtherKind { member, kind };
        match member {
            PatchMember::ObservedAt => self.meta_mut().observed_at = Some(MemberChange::Delete),
            PatchMember::ExpiresAt => self.meta_mut().expires_at = Some(MemberChange::Delete),
            PatchMember::UnitCode => {
                let slot = match &mut self {
                    Self::Property(property) => &mut property.unit_code,
                    Self::GeoProperty(property) => &mut property.unit_code,
                    Self::ListProperty(property) => &mut property.unit_code,
                    Self::LanguageProperty(_) | Self::VocabProperty(_) | Self::JsonProperty(_) | Self::Relationship(_) | Self::ListRelationship(_) => {
                        return Err(refused);
                    }
                };
                *slot = Some(MemberChange::Delete);
            }
            PatchMember::ValueType => {
                let slot = match &mut self {
                    Self::Property(property) => &mut property.value_type,
                    Self::GeoProperty(property) => &mut property.value_type,
                    Self::LanguageProperty(property) => &mut property.value_type,
                    Self::VocabProperty(property) => &mut property.value_type,
                    Self::ListProperty(property) => &mut property.value_type,
                    Self::JsonProperty(property) => &mut property.value_type,
                    Self::Relationship(_) | Self::ListRelationship(_) => return Err(refused),
                };
                *slot = Some(MemberChange::Delete);
            }
            PatchMember::ObjectType => {
                let slot = match &mut self {
                    Self::Relationship(relationship) => &mut relationship.object_type,
                    Self::ListRelationship(relationship) => &mut relationship.object_type,
                    Self::Property(_)
                    | Self::GeoProperty(_)
                    | Self::LanguageProperty(_)
                    | Self::VocabProperty(_)
                    | Self::ListProperty(_)
                    | Self::JsonProperty(_) => return Err(refused),
                };
                *slot = Some(MemberChange::Delete);
            }
        }
        Ok(self)
    }

    /// The same fragment, deleting the sub-attribute `name` of the target attribute with `deletion`: the bare
    /// NGSI-LD Null ([`Deletion::Member`], clause 5.4 example 2), or, for an instance of a known kind, an object whose
    /// payload is the Null ([`InstanceDeletion`](crate::model::InstanceDeletion)), as
    /// `"hasWeatherObserved": {"type": "Relationship", "object": "urn:ngsi-ld:null"}`.
    ///
    /// ```
    /// use ngsi_ld::model::{Attribute, AttributeKind, InstanceDeletion, Lossless, Property};
    ///
    /// let level = Attribute::from(Property::builder(4).build()?)
    ///     .into_patch()
    ///     .delete_sub_attribute(
    ///         "hasWeatherObserved",
    ///         InstanceDeletion::new(AttributeKind::Relationship),
    ///     )?;
    /// assert_eq!(
    ///     level.render(Lossless::Normalized).to_string(),
    ///     concat!(
    ///         r#"{"type":"Property","value":4,"#,
    ///         r#""hasWeatherObserved":{"type":"Relationship","#,
    ///         r#""object":"urn:ngsi-ld:null"}}"#
    ///     )
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`BuildError`] when `name` is not an attribute name, or the fragment already holds a change of the
    /// sub-attribute the deletion cannot stand beside: the bare Null among other instances, or a second change of the
    /// same instance.
    pub fn delete_sub_attribute(mut self, name: impl Into<AttributeNameDraft>, deletion: impl Into<Deletion>) -> Result<Self, BuildError> {
        let mut problems = Vec::new();
        match resolve_name(0, name.into()) {
            Ok(name) => {
                if let Err(problem) = insert(&mut self.meta_mut().attributes, name, Change::Delete(deletion.into())) {
                    problems.push(BuildProblem::top(problem));
                }
            }
            Err(problem) => problems.push(BuildProblem::top(problem)),
        }
        BuildError::finish(problems, self)
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{
        Attribute,
        AttributeKind,
        Change,
        EntityId,
        EntityTypeName,
        GeoProperty,
        InstanceDeletion,
        ListProperty,
        Lossless,
        MemberChange,
        MemberOfOtherKind,
        Patch,
        PatchMember,
        Property,
        Relationship,
        Value,
        geometry::{Position, Shape},
    };
    use cefact_units::UnitCode;
    use iri_rs::iri;
    use pretty_assertions::assert_eq;

    fn level() -> Attribute<Patch> {
        Attribute::from(
            Property::builder(4)
                .unit_code(UnitCode::C62)
                .observed_at("2020-09-09T16:40:00Z".parse().unwrap())
                .build()
                .unwrap(),
        )
        .into_patch()
    }

    fn sensor() -> Attribute<Patch> {
        let sensor_type: EntityTypeName = "Sensor".parse().unwrap();
        Attribute::from(
            Relationship::builder(EntityId::from(iri!("urn:ngsi-ld:Sensor:1")))
                .object_type(sensor_type)
                .build()
                .unwrap(),
        )
        .into_patch()
    }

    #[test]
    fn a_deleted_member_replaces_the_value_the_fragment_set_and_writes_the_null_in_its_place() {
        let deleted = level().delete_member(PatchMember::UnitCode).unwrap();
        let Attribute::Property(property) = &deleted else {
            panic!("a Property stays a Property");
        };
        assert_eq!(property.unit_code(), Some(MemberChange::Delete));
        assert_eq!(
            deleted.render(Lossless::Normalized).to_string(),
            r#"{"type":"Property","value":4,"unitCode":"urn:ngsi-ld:null","observedAt":"2020-09-09T16:40:00Z"}"#
        );
        let both = deleted
            .delete_member(PatchMember::ObservedAt)
            .unwrap()
            .delete_member(PatchMember::ExpiresAt)
            .unwrap();
        assert_eq!(
            both.render(Lossless::Concise).to_string(),
            r#"{"value":4,"unitCode":"urn:ngsi-ld:null","observedAt":"urn:ngsi-ld:null","expiresAt":"urn:ngsi-ld:null"}"#
        );
    }

    #[test]
    fn every_member_a_kind_has_can_be_deleted() {
        let relationship = sensor().delete_member(PatchMember::ObjectType).unwrap();
        assert_eq!(
            relationship.render(Lossless::Normalized).to_string(),
            r#"{"type":"Relationship","objectType":"urn:ngsi-ld:null","object":"urn:ngsi-ld:Sensor:1"}"#
        );
        let list = Attribute::from(ListProperty::builder(vec![Value::from(1), Value::from(2)]).build().unwrap())
            .into_patch()
            .delete_member(PatchMember::UnitCode)
            .and_then(|list| list.delete_member(PatchMember::ValueType))
            .unwrap();
        assert_eq!(
            list.render(Lossless::Normalized).to_string(),
            r#"{"type":"ListProperty","valueList":[1,2],"valueType":"urn:ngsi-ld:null","unitCode":"urn:ngsi-ld:null"}"#
        );
        let geo = Attribute::from(GeoProperty::builder(Shape::Point(Position::try_new(1.0, 2.0).unwrap())).build().unwrap())
            .into_patch()
            .delete_member(PatchMember::UnitCode)
            .unwrap();
        assert!(geo.render(Lossless::Normalized).to_string().contains(r#""unitCode":"urn:ngsi-ld:null""#));
    }

    #[test]
    fn a_member_the_kind_does_not_have_is_refused() {
        assert_eq!(
            sensor().delete_member(PatchMember::UnitCode),
            Err(MemberOfOtherKind {
                member: PatchMember::UnitCode,
                kind: AttributeKind::Relationship
            })
        );
        assert_eq!(
            sensor().delete_member(PatchMember::ValueType).unwrap_err().to_string(),
            "a Relationship has no `valueType` member"
        );
        assert_eq!(
            level().delete_member(PatchMember::ObjectType),
            Err(MemberOfOtherKind {
                member: PatchMember::ObjectType,
                kind: AttributeKind::Property
            })
        );
    }

    #[test]
    fn a_deleted_member_reads_back_as_a_deletion() {
        let deleted = || {
            level()
                .delete_member(PatchMember::ObservedAt)
                .and_then(|level| level.delete_member(PatchMember::UnitCode))
                .unwrap()
        };
        for form in [Lossless::Normalized, Lossless::Concise] {
            let text = deleted().render(form).to_string();
            let read = Change::<Attribute<Patch>, InstanceDeletion>::parse(&text, form).unwrap();
            assert_eq!(read, Change::Set(deleted()), "{text}");
        }
    }
}
