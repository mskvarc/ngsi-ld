//! The temporal evolution of an entity's scope (NGSI-LD clause 4.5.6): a temporal Property whose only
//! sub-Properties are `observedAt`, `createdAt`, `modifiedAt` and `deletedAt`.

use crate::model::{
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
    Scope,
    entity::member::SystemMember,
    keyword_alias::unalias_attribute,
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
    /// The scope was removed: the NGSI-LD Null a broker records for it (clause 4.5.0). A request holds it only
    /// beside the `deletedAt` of the removal (clauses 4.5.6 and 4.18).
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
/// or deleted the instance (NGSI-LD clause 4.5.6). A request may record the removal of a scope with the time it
/// happened, as it may record the deletion of an attribute instance (clause 4.18).
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
///
/// let removed = ScopeInstance::removed("2018-08-01T12:00:00Z".parse()?);
/// assert!(removed.value().is_removed());
/// assert!(removed.deleted_at().is_some());
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
    /// The `deletedAt` of a removal a request records, set exactly when the value is [`ScopeValue::Removed`]; `()`
    /// in a response.
    removal: D::ScopeRemoval,
}

impl ScopeInstance<Input> {
    /// An instance setting one scope or several, with no time yet.
    #[must_use]
    pub fn new(scopes: impl Into<OneOrMany<Scope>>) -> Self {
        Self {
            value: ScopeValue::Scopes(scopes.into()),
            observed_at: None,
            output: (),
            removal: None,
        }
    }

    /// An instance recording that the scope was removed at `deleted_at` (clauses 4.5.6 and 4.18): the NGSI-LD Null
    /// with the `deletedAt` of the removal, which a request may carry although it carries no other Null (clause
    /// 5.5.4).
    #[must_use]
    pub const fn removed(deleted_at: DateTime) -> Self {
        Self {
            value: ScopeValue::Removed,
            observed_at: None,
            output: (),
            removal: Some(deleted_at),
        }
    }

    /// Returns when the scope was removed (`deletedAt`), for an instance made by [`removed`](Self::removed).
    #[must_use]
    pub const fn deleted_at(&self) -> Option<&DateTime> {
        self.removal.as_ref()
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
            removal: (),
        }
    }
}

impl ScopeInstance<Output> {
    /// Drops the members only a broker writes. A removed scope keeps the `deletedAt` the broker recorded for it; one
    /// without it holds the NGSI-LD Null a request cannot carry (clause 5.5.4), so it is recorded among `problems`
    /// and dropped.
    pub(crate) fn into_input(self, problems: &mut Vec<BuildProblem>) -> Option<ScopeInstance<Input>> {
        match (self.value, self.output.deleted_at) {
            (ScopeValue::Scopes(scopes), _) => Some(ScopeInstance {
                value: ScopeValue::Scopes(scopes),
                observed_at: self.observed_at,
                output: (),
                removal: None,
            }),
            (ScopeValue::Removed, Some(deleted_at)) => Some(ScopeInstance {
                value: ScopeValue::Removed,
                observed_at: self.observed_at,
                output: (),
                removal: Some(deleted_at),
            }),
            (ScopeValue::Removed, None) => {
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
    D::write_scope_removal(&instance.removal, &mut object);
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
    let object = unalias_attribute(object(json)?)?;
    let mut typed = None;
    let mut value = None;
    let mut observed_at = None;
    let mut output = D::EntityOutput::default();
    let mut removal = D::ScopeRemoval::default();
    for Entry { key, value: member } in object {
        let key = key.as_str();
        match key {
            "type" => once(&mut typed, key, scope_type(member)?)?,
            "value" => once(&mut value, key, ScopeValue::from_json(member)?)?,
            "observedAt" => once(&mut observed_at, key, date_time(member, "observedAt")?)?,
            other => match other.parse::<SystemMember>() {
                Ok(system) => D::read_scope_member(&mut output, &mut removal, system, member)?,
                Err(_) => return Err(unexpected(other)),
            },
        }
    }
    if form == Lossless::Normalized && typed.is_none() {
        return Err(DocumentError::top(DocumentErrorKind::MissingType));
    }
    let value = required(value, "value")?;
    // A removed scope is the NGSI-LD Null of a Property value, which the direction admits or refuses, as it does for
    // any attribute (clause 5.5.4); a request admits it beside the `deletedAt` of the removal.
    D::admit_scope_value(&value, &removal)?;
    Ok(ScopeInstance {
        value,
        observed_at,
        output,
        removal,
    })
}

/// Reads the `type` of a scope instance, which clause 4.5.6 makes a Property.
fn scope_type(json: Json) -> Result<(), DocumentError> {
    match json {
        Json::String(text) if text.as_str() == "Property" => Ok(()),
        Json::String(text) => Err(DocumentError::top(DocumentErrorKind::ScopeNotAProperty { found: text.into_string() })),
        Json::Null | Json::Boolean(_) | Json::Number(_) | Json::Array(_) | Json::Object(_) => Err(DocumentError::top(DocumentErrorKind::MissingType)),
    }
}
