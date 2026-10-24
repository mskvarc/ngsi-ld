//! The temporal evolution of an entity's scope (NGSI-LD clause 4.5.6): a temporal Property whose only
//! sub-Properties are `observedAt`, `createdAt`, `modifiedAt` and `deletedAt`.

use crate::model::{
    Attribute,
    BuildProblem,
    DateTime,
    DocumentError,
    DocumentErrorKind,
    EntityDirection,
    EntityOutputMembers,
    Input,
    Lossless,
    NullMember,
    OneOrMany,
    Output,
    ProblemKind,
    Property,
    Scope,
    Value,
    entity::member::SystemMember,
    null::NGSI_LD_NULL,
    reader::{date_time, empty, object, once, required, scopes, unexpected},
    representation::{
        read::direction::ReadEntityDirection,
        write::{direction::WriteEntityDirection, push_display},
    },
};
use bon::bon;
use jstrict::{Object, Value as Json, object::Entry};

/// The value of one instance of a scope's evolution.
///
/// The enum is exhaustive: a scope is set or removed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::IsVariant)]
pub enum ScopeValue {
    /// One scope, or several (clause 4.18).
    Scopes(OneOrMany<Scope>),
    /// The scope was removed: the NGSI-LD Null a broker records for it (clause 4.5.0), which only a response
    /// holds.
    Removed,
}

impl ScopeValue {
    /// Writes the value: the scope or scopes, or the NGSI-LD Null.
    pub(crate) fn to_json(&self) -> Json {
        match self {
            Self::Scopes(scopes) => scopes.to_json(|scope| Json::from(scope.to_string())),
            Self::Removed => Json::from(NGSI_LD_NULL.as_str()),
        }
    }

    /// Reads the value: the NGSI-LD Null, or one scope or a non-empty array of them.
    pub(crate) fn from_json(json: Json) -> Result<Self, DocumentError> {
        match json {
            Json::String(text) if text.as_str() == NGSI_LD_NULL.as_str() => Ok(Self::Removed),
            other @ (Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Array(_) | Json::Object(_)) => scopes(other).map(Self::Scopes),
        }
    }
}

/// One instance of the temporal evolution of an entity's scope.
///
/// It holds the scope or scopes it held, when that was observed, and, in a response, when the broker created, modified
/// or deleted the instance (NGSI-LD clause 4.5.6).
///
/// ```
/// use ngsi_ld::model::{
///     Scope,
///     evolution::{ScopeInstance, ScopeValue},
/// };
///
/// let centre = ScopeInstance::new("/Madrid/Centro".parse::<Scope>()?)
///     .with_observed_at("2018-08-01T11:00:00Z".parse()?);
/// assert!(matches!(centre.value(), ScopeValue::Scopes(_)));
/// assert!(centre.observed_at().is_some());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeInstance<D: EntityDirection> {
    /// `value`.
    value: ScopeValue,
    /// `observedAt`.
    observed_at: Option<DateTime>,
    /// `createdAt`, `modifiedAt` and `deletedAt`; `()` in a request.
    output: D::EntityOutput,
}

impl ScopeInstance<Input> {
    /// An instance setting one scope or several, with no time yet.
    #[must_use]
    pub fn new(scopes: impl Into<OneOrMany<Scope>>) -> Self {
        Self {
            value: ScopeValue::Scopes(scopes.into()),
            observed_at: None,
            output: (),
        }
    }

    /// Records when the scope was observed (`observedAt`).
    #[must_use]
    pub fn with_observed_at(self, time: DateTime) -> Self {
        Self {
            observed_at: Some(time),
            ..self
        }
    }
}

#[bon]
impl ScopeInstance<Output> {
    /// Starts the builder of an instance as a broker returns it: the scope or scopes it held, or
    /// [`ScopeValue::Removed`] for a removed scope, when that was observed, and the system members of the instance
    /// (clause 4.5.6).
    ///
    /// ```
    /// use ngsi_ld::model::evolution::{ScopeInstance, ScopeValue};
    ///
    /// let removed = ScopeInstance::output_builder(ScopeValue::Removed)
    ///     .observed_at("2018-08-01T11:00:00Z".parse()?)
    ///     .build();
    /// assert!(removed.value().is_removed());
    /// assert!(removed.output().created_at.is_none());
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[builder(builder_type = ScopeInstanceOutputBuilder, finish_fn = build)]
    pub const fn output_builder(
        #[builder(start_fn)] value: ScopeValue,
        observed_at: Option<DateTime>,
        created_at: Option<DateTime>,
        modified_at: Option<DateTime>,
        deleted_at: Option<DateTime>,
    ) -> Self {
        Self {
            value,
            observed_at,
            output: EntityOutputMembers {
                created_at,
                modified_at,
                deleted_at,
            },
        }
    }
}

impl ScopeInstance<Output> {
    /// Drops the members only a broker writes; a removed scope, which holds the NGSI-LD Null a request cannot carry
    /// (clause 5.5.4), is recorded among `problems` and dropped.
    pub(crate) fn into_input(self, problems: &mut Vec<BuildProblem>) -> Option<ScopeInstance<Input>> {
        match self.value {
            ScopeValue::Scopes(scopes) => Some(ScopeInstance {
                value: ScopeValue::Scopes(scopes),
                observed_at: self.observed_at,
                output: (),
            }),
            ScopeValue::Removed => {
                problems.push(BuildProblem::top(ProblemKind::NgsiLdNull { member: NullMember::Scope }));
                None
            }
        }
    }
}

impl<D: EntityDirection> ScopeInstance<D> {
    /// Returns the scope or scopes, or [`ScopeValue::Removed`].
    #[must_use]
    pub const fn value(&self) -> &ScopeValue {
        &self.value
    }

    /// Returns when the scope was observed (`observedAt`).
    #[must_use]
    pub const fn observed_at(&self) -> Option<&DateTime> {
        self.observed_at.as_ref()
    }

    /// Returns the members only a broker writes: `createdAt`, `modifiedAt` and `deletedAt` in a response, `()` in
    /// a request.
    #[must_use]
    pub const fn output(&self) -> &D::EntityOutput {
        &self.output
    }
}

/// Writes one instance: `type` in normalized form, `value`, `observedAt`, then the members only a broker writes.
pub(crate) fn write_scope_instance<D: WriteEntityDirection>(instance: &ScopeInstance<D>, form: Lossless) -> Json {
    let mut object = Object::new();
    if form == Lossless::Normalized {
        object.push("type".into(), Json::from("Property"));
    }
    object.push("value".into(), instance.value.to_json());
    push_display(&mut object, "observedAt", instance.observed_at.as_ref());
    D::write_entity_output(&instance.output, &mut object);
    Json::Object(object)
}

/// Reads the evolution of a scope: an array of instances, or the single instance clause 4.5.7 allows when only one
/// is present.
pub(crate) fn read_scope_evolution<D: ReadEntityDirection>(json: Json, form: Lossless) -> Result<Vec<ScopeInstance<D>>, DocumentError> {
    let items = match json {
        Json::Array(items) if items.is_empty() => return Err(empty()),
        Json::Array(items) => items,
        single @ (Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Object(_)) => vec![single].into(),
    };
    items.into_iter().map(|item| read_scope_instance::<D>(item, form)).collect()
}

/// Reads one instance: a Property in normalized form (`type` required) or concise form (`type` optional), with no
/// member besides `value`, `observedAt` and the system members of the direction.
fn read_scope_instance<D: ReadEntityDirection>(json: Json, form: Lossless) -> Result<ScopeInstance<D>, DocumentError> {
    let object = object(json)?;
    let mut typed = None;
    let mut value = None;
    let mut observed_at = None;
    let mut output = D::EntityOutput::default();
    for Entry { key, value: member } in object {
        let key = key.as_str();
        match key {
            "type" => once(&mut typed, key, scope_type(member)?)?,
            "value" => once(&mut value, key, ScopeValue::from_json(member)?)?,
            "observedAt" => once(&mut observed_at, key, date_time(member, "observedAt")?)?,
            other => match other.parse::<SystemMember>() {
                Ok(system) => D::read_entity_output(&mut output, system, member)?,
                Err(_) => return Err(unexpected(other)),
            },
        }
    }
    if form == Lossless::Normalized && typed.is_none() {
        return Err(DocumentError::top(DocumentErrorKind::MissingType));
    }
    let value = required(value, "value")?;
    if value.is_removed() {
        // A removed scope is the NGSI-LD Null of a Property value, which the direction admits or refuses, as it does
        // for any attribute (clause 5.5.4).
        D::admit(&Attribute::Property(Property::plain(Value::Text(NGSI_LD_NULL.as_str().to_owned()))))?;
    }
    Ok(ScopeInstance { value, observed_at, output })
}

/// Reads the `type` of a scope instance, which clause 4.5.6 makes a Property.
fn scope_type(json: Json) -> Result<(), DocumentError> {
    match json {
        Json::String(text) if text.as_str() == "Property" => Ok(()),
        Json::String(text) => Err(DocumentError::top(DocumentErrorKind::ScopeNotAProperty { found: text.into_string() })),
        Json::Null | Json::Boolean(_) | Json::Number(_) | Json::Array(_) | Json::Object(_) => Err(DocumentError::top(DocumentErrorKind::MissingType)),
    }
}
