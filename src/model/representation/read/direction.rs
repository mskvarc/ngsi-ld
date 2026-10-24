//! What each readable direction makes of the members only a broker writes, and of the NGSI-LD Null.

use crate::model::{
    Attribute,
    Direction,
    DocumentError,
    DocumentErrorKind,
    EntityDirection,
    EntityOutputMembers,
    Input,
    InstanceId,
    Lossless,
    Output,
    entity::member::SystemMember,
    null,
    reader::{date_time, duplicate, identifier, once, unexpected},
    representation::{
        kind_output::{KindOutputMember, KindOutputRepresentation},
        read::read_instance,
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

    /// Takes the `instanceId` a request names for an instance of a temporal evolution out of the instance, before the
    /// instance is read as an attribute; a response leaves it among the instance's output-only members.
    fn take_temporal_instance_id(instance: &mut Json) -> Result<Self::TemporalInstanceId, DocumentError>;
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

    fn take_temporal_instance_id(instance: &mut Json) -> Result<Option<InstanceId>, DocumentError> {
        let Json::Object(object) = instance else {
            return Ok(None);
        };
        let name = <&str>::from(OutputMember::InstanceId);
        object
            .remove_unique(name)
            .map_err(|_| duplicate(name))?
            .map(|entry| identifier(entry.value, name))
            .transpose()
    }
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

    fn take_temporal_instance_id(_: &mut Json) -> Result<(), DocumentError> {
        Ok(())
    }
}
