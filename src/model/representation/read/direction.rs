//! What each readable direction makes of the members only a broker writes, and of the NGSI-LD Null.

use crate::model::{
    Attribute,
    AttributeName,
    DateTime,
    Direction,
    DocumentError,
    DocumentErrorKind,
    EntityDirection,
    EntityOutputMembers,
    Input,
    Lossless,
    Output,
    Property,
    Value,
    entity::member::SystemMember,
    evolution::{ScopeValue, TemporalAttributes, TemporalInstanceMembers},
    null::{self, NGSI_LD_NULL},
    reader::{date_time, duplicate, empty, identifier, once, unexpected},
    representation::{
        kind_output::{KindOutputMember, KindOutputRepresentation},
        read::{read_instance, read_unadmitted},
    },
};
use jstrict::Value as Json;
use std::fmt::Debug;

/// A member of an attribute only a broker writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumString, strum::IntoStaticStr)]
pub(crate) enum OutputMember {
    /// `createdAt`.
    #[strum(serialize = "createdAt")]
    CreatedAt,
    /// `modifiedAt`.
    #[strum(serialize = "modifiedAt")]
    ModifiedAt,
    /// `deletedAt`.
    #[strum(serialize = "deletedAt")]
    DeletedAt,
    /// `instanceId`.
    #[strum(serialize = "instanceId")]
    InstanceId,
}

/// How each readable direction treats the members only a broker writes, and the NGSI-LD Null.
pub(crate) trait ReadDirection: Direction {
    /// Reads an optional member of an attribute with `read`, or, in a fragment, the NGSI-LD Null that deletes it
    /// (clause 5.5.8).
    fn read_member<T: Debug + Clone + PartialEq + Eq>(
        json: Option<Json>,
        read: impl FnOnce(Json) -> Result<T, DocumentError>,
    ) -> Result<Self::Member<T>, DocumentError>;

    /// Reads an output-only attribute member, or refuses it.
    fn read_attribute_output(output: &mut Self::AttributeOutput, member: OutputMember, json: Json) -> Result<(), DocumentError>;

    /// Reads the output-only members of one kind, `K`, found on an instance, or refuses them.
    fn read_kind_output<K: KindOutputRepresentation>(members: Vec<(KindOutputMember, Json)>, form: Lossless) -> Result<Self::KindOutput<K>, DocumentError>;

    /// Admits an attribute instance read in this direction, or refuses it.
    fn admit(attribute: &Attribute<Self>) -> Result<(), DocumentError>;

    /// Reads one instance of an attribute member: the attribute, or in a fragment a change to it.
    fn read_slot(json: Json, form: Lossless) -> Result<Self::Slot, DocumentError>;
}

/// How each entity direction treats the entity members only a broker writes.
pub(crate) trait ReadEntityDirection: ReadDirection + EntityDirection {
    /// Reads `createdAt`, `modifiedAt` or `deletedAt`, or refuses it.
    fn read_entity_output(output: &mut Self::EntityOutput, member: SystemMember, json: Json) -> Result<(), DocumentError>;

    /// Reads `createdAt`, `modifiedAt` or `deletedAt` of one instance of the evolution of `scope`. A request records
    /// the `deletedAt` of a removed scope in `removal` and refuses the other two; a response keeps all three in
    /// `output`.
    fn read_scope_member(output: &mut Self::EntityOutput, removal: &mut Self::ScopeRemoval, member: SystemMember, json: Json) -> Result<(), DocumentError>;

    /// Admits the value of one instance of the evolution of `scope`. A request holds the NGSI-LD Null, which clause
    /// 5.5.4 otherwise keeps out of it, only for a removed scope that records its `deletedAt` (clauses 4.5.6 and
    /// 4.18); a response admits both.
    fn admit_scope_value(value: &ScopeValue, removal: &Self::ScopeRemoval) -> Result<(), DocumentError>;

    /// Reads one instance of a temporal evolution. A request may name its `instanceId` and record a deletion with
    /// `deletedAt` (clauses 4.5.7 and 4.5.8), which are taken out of the instance before it is read as an attribute;
    /// a response leaves them among the instance's output-only members.
    fn read_temporal_instance(instance: Json, form: Lossless) -> Result<(Attribute<Self>, Self::TemporalMembers), DocumentError>;

    /// Reads an attribute whose evolution is an empty array. A request carries nothing by it and refuses it; a
    /// response may hold it, since the instances of a page of a paginated answer are the ones of its time range
    /// (clause 6.3.10) and an attribute may have none in it.
    fn read_empty_evolution(attributes: &mut TemporalAttributes<Self>, name: AttributeName) -> Result<(), DocumentError>;
}

impl ReadDirection for Input {
    fn read_member<T: Debug + Clone + PartialEq + Eq>(
        json: Option<Json>,
        read: impl FnOnce(Json) -> Result<T, DocumentError>,
    ) -> Result<Option<T>, DocumentError> {
        json.map(read).transpose()
    }

    fn read_attribute_output((): &mut Self::AttributeOutput, member: OutputMember, _: Json) -> Result<(), DocumentError> {
        Err(unexpected(<&str>::from(member)))
    }

    fn read_kind_output<K: KindOutputRepresentation>(members: Vec<(KindOutputMember, Json)>, _: Lossless) -> Result<(), DocumentError> {
        members.first().map_or(Ok(()), |&(member, _)| Err(unexpected(member.as_str())))
    }

    /// Clause 5.5.4 refuses the NGSI-LD Null in a request outside fragments.
    fn admit(attribute: &Attribute<Self>) -> Result<(), DocumentError> {
        null::check_attribute(attribute).map_err(|member| DocumentError::top(DocumentErrorKind::NgsiLdNull { member }))
    }

    fn read_slot(json: Json, form: Lossless) -> Result<Self::Slot, DocumentError> {
        read_instance::<Self>(json, form)
    }
}

impl ReadEntityDirection for Input {
    fn read_entity_output((): &mut Self::EntityOutput, member: SystemMember, _: Json) -> Result<(), DocumentError> {
        Err(unexpected(member.as_str()))
    }

    fn read_scope_member((): &mut Self::EntityOutput, removal: &mut Option<DateTime>, member: SystemMember, json: Json) -> Result<(), DocumentError> {
        match member {
            SystemMember::Deleted => once(removal, member.as_str(), date_time(json, "deletedAt")?),
            SystemMember::Created | SystemMember::Modified => Err(unexpected(member.as_str())),
        }
    }

    /// A removed scope is the NGSI-LD Null, so it must record its `deletedAt`, and `deletedAt` beside scopes is not a
    /// member of the instance.
    fn admit_scope_value(value: &ScopeValue, removal: &Option<DateTime>) -> Result<(), DocumentError> {
        match (value, removal) {
            (ScopeValue::Removed, None) => Self::admit(&Attribute::Property(Property::plain(Value::Text(NGSI_LD_NULL.as_str().to_owned())))),
            (ScopeValue::Scopes(_), Some(_)) => Err(unexpected(SystemMember::Deleted.as_str())),
            (ScopeValue::Removed, Some(_)) | (ScopeValue::Scopes(_), None) => Ok(()),
        }
    }

    /// An instance whose `deletedAt` is given records a deletion, so its payload must be the NGSI-LD Null of its kind
    /// (clause 4.5.0), which clause 5.5.4 otherwise keeps out of a request; `deletedAt` beside any other payload is not
    /// a member of the instance.
    fn read_temporal_instance(mut instance: Json, form: Lossless) -> Result<(Attribute<Input>, TemporalInstanceMembers), DocumentError> {
        let instance_id = take_member(&mut instance, <&str>::from(OutputMember::InstanceId), |json| identifier(json, "instanceId"))?;
        let deleted_at = take_member(&mut instance, <&str>::from(OutputMember::DeletedAt), |json| date_time(json, "deletedAt"))?;
        let Some(deleted_at) = deleted_at else {
            return Ok((read_instance::<Input>(instance, form)?, TemporalInstanceMembers::new(instance_id, None)));
        };
        let attribute = read_unadmitted::<Input>(instance, form)?;
        if !null::is_null_payload(&attribute) {
            return Err(unexpected(<&str>::from(OutputMember::DeletedAt)));
        }
        Ok((attribute, TemporalInstanceMembers::new(instance_id, Some(deleted_at))))
    }

    fn read_empty_evolution(_: &mut TemporalAttributes<Self>, _: AttributeName) -> Result<(), DocumentError> {
        Err(empty())
    }
}

/// Takes the member `name` out of `instance` and reads it with `read`; `None` when the instance is not an object or
/// has no such member.
fn take_member<T>(instance: &mut Json, name: &'static str, read: impl FnOnce(Json) -> Result<T, DocumentError>) -> Result<Option<T>, DocumentError> {
    let Json::Object(object) = instance else {
        return Ok(None);
    };
    object
        .remove_unique(name)
        .map_err(|_| duplicate(name))?
        .map(|entry| read(entry.value))
        .transpose()
}

impl ReadDirection for Output {
    fn read_member<T: Debug + Clone + PartialEq + Eq>(
        json: Option<Json>,
        read: impl FnOnce(Json) -> Result<T, DocumentError>,
    ) -> Result<Option<T>, DocumentError> {
        json.map(read).transpose()
    }

    fn read_attribute_output(output: &mut Self::AttributeOutput, member: OutputMember, json: Json) -> Result<(), DocumentError> {
        let name = <&str>::from(member);
        match member {
            OutputMember::CreatedAt => once(&mut output.created_at, name, date_time(json, "createdAt")?),
            OutputMember::ModifiedAt => once(&mut output.modified_at, name, date_time(json, "modifiedAt")?),
            OutputMember::DeletedAt => once(&mut output.deleted_at, name, date_time(json, "deletedAt")?),
            OutputMember::InstanceId => once(&mut output.instance_id, name, identifier(json, "instanceId")?),
        }
    }

    fn read_kind_output<K: KindOutputRepresentation>(members: Vec<(KindOutputMember, Json)>, form: Lossless) -> Result<K, DocumentError> {
        members.into_iter().try_fold(K::default(), |mut output, (member, json)| {
            output.read(member, json, form)?;
            Ok(output)
        })
    }

    /// A broker writes the NGSI-LD Null for a deleted attribute in notifications and temporal evolutions
    /// (clause 4.5.0), so a response may hold it.
    fn admit(_: &Attribute<Self>) -> Result<(), DocumentError> {
        Ok(())
    }

    fn read_slot(json: Json, form: Lossless) -> Result<Self::Slot, DocumentError> {
        read_instance::<Self>(json, form)
    }
}

impl ReadEntityDirection for Output {
    fn read_entity_output(output: &mut EntityOutputMembers, member: SystemMember, json: Json) -> Result<(), DocumentError> {
        let name = member.as_str();
        match member {
            SystemMember::Created => once(&mut output.created_at, name, date_time(json, "createdAt")?),
            SystemMember::Modified => once(&mut output.modified_at, name, date_time(json, "modifiedAt")?),
            SystemMember::Deleted => once(&mut output.deleted_at, name, date_time(json, "deletedAt")?),
        }
    }

    fn read_scope_member(output: &mut EntityOutputMembers, (): &mut (), member: SystemMember, json: Json) -> Result<(), DocumentError> {
        Self::read_entity_output(output, member, json)
    }

    fn admit_scope_value(_: &ScopeValue, (): &()) -> Result<(), DocumentError> {
        Ok(())
    }

    fn read_temporal_instance(instance: Json, form: Lossless) -> Result<(Attribute<Output>, ()), DocumentError> {
        Ok((read_instance::<Output>(instance, form)?, ()))
    }

    fn read_empty_evolution(attributes: &mut TemporalAttributes<Self>, name: AttributeName) -> Result<(), DocumentError> {
        attributes.insert_empty(name);
        Ok(())
    }
}
