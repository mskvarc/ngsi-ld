//! JSON-LD value objects with an explicit datatype, the form clause C.6 writes Date, Time and `DateTime` Property
//! values in.

use crate::model::{
    Date,
    DateTime,
    TemporalError,
    Time,
    VocabTerm,
    keyword::{TYPE, VALUE},
};
use jstrict::{Object, Value as Json};

/// A JSON-LD value object with an explicit datatype.
///
/// NGSI-LD clause C.6 uses this form for Date, Time and `DateTime` Property values.
///
/// A core datatype always carries a lexical form of that datatype (clause 4.6.3), so `DateTime` with
/// `"yesterday"` cannot be represented.
///
/// ```
/// use ngsi_ld::model::{CoreDatatype, Datatype, TypedValue};
///
/// let tested = TypedValue::try_new(CoreDatatype::DateTime, "2018-12-04T12:00:00Z")?;
/// assert_eq!(tested.datatype(), &Datatype::Core(CoreDatatype::DateTime));
/// assert!(TypedValue::try_new(CoreDatatype::Date, "yesterday").is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypedValue {
    /// The datatype the lexical form is read as.
    datatype: Datatype,
    /// The lexical form as written; valid for `datatype` when that is a core datatype.
    lexical: String,
}

/// The lexical form of a typed value is not one of its core datatype.
///
/// NGSI-LD clause 4.6.3.
///
/// The struct is `#[non_exhaustive]`: its fields are readable, and only the crate builds it.
#[derive(Debug, Clone, thiserror::Error)]
#[error("not a lexical form of {datatype}")]
#[non_exhaustive]
pub struct TypedValueError {
    /// The datatype the value names.
    pub datatype: CoreDatatype,
    /// Why the lexical form is refused.
    #[source]
    pub source: TemporalError,
}

impl TypedValue {
    /// A typed value. A datatype given as a term that names a core datatype (`DateTime`, `Date`, `Time`) is that
    /// core datatype.
    ///
    /// # Errors
    ///
    /// Returns [`TypedValueError`] when the datatype is a core one and `lexical` is not a lexical form of it.
    pub fn try_new(datatype: impl Into<Datatype>, lexical: impl Into<String>) -> Result<Self, TypedValueError> {
        let datatype = match datatype.into() {
            Datatype::Other(term) => Datatype::from(term),
            core @ Datatype::Core(_) => core,
        };
        let lexical = lexical.into();
        if let Datatype::Core(core) = datatype {
            core.check(&lexical).map_err(|source| TypedValueError { datatype: core, source })?;
        }
        Ok(Self { datatype, lexical })
    }

    /// Returns the datatype (`@type`).
    #[must_use]
    pub const fn datatype(&self) -> &Datatype {
        &self.datatype
    }

    /// Returns the lexical form (`@value`).
    #[must_use]
    pub fn lexical(&self) -> &str {
        &self.lexical
    }

    /// A typed value of a core datatype whose lexical form is known to be valid, such as the rendering of a
    /// [`DateTime`].
    pub(crate) const fn of_core(datatype: CoreDatatype, lexical: String) -> TypedValue {
        TypedValue {
            datatype: Datatype::Core(datatype),
            lexical,
        }
    }

    /// Reads the datatype and the `@value` of a value object as a typed value when the datatype is a term and the
    /// `@value` a string; `None` otherwise.
    pub(crate) fn from_members(datatype: &Json, lexical: &Json) -> Option<Result<TypedValue, TypedValueError>> {
        let datatype = VocabTerm::try_new(datatype.as_str()?).ok()?;
        Some(TypedValue::try_new(datatype, lexical.as_str()?))
    }

    /// Writes the typed value as a JSON-LD value object, `@type` first.
    pub(crate) fn to_json(&self) -> Json {
        let mut object = Object::new();
        object.push(TYPE.into(), Json::from(self.datatype.to_string()));
        object.push(VALUE.into(), Json::from(self.lexical.as_str()));
        Json::Object(object)
    }
}

/// The datatypes of the core `@context` for typed Property values.
///
/// NGSI-LD clause C.6 writes typed Property values with them. The set is the one clause 4.6.3 gives lexical forms for,
/// so the enum is exhaustive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr)]
pub enum CoreDatatype {
    /// `DateTime`.
    DateTime,
    /// `Date`.
    Date,
    /// `Time`.
    Time,
}

impl CoreDatatype {
    /// Checks that `lexical` is a lexical form of the datatype.
    fn check(self, lexical: &str) -> Result<(), TemporalError> {
        match self {
            Self::DateTime => lexical.parse::<DateTime>().map(drop),
            Self::Date => lexical.parse::<Date>().map(drop),
            Self::Time => lexical.parse::<Time>().map(drop),
        }
    }
}

/// The datatype of a [`TypedValue`].
///
/// The enum is exhaustive: a datatype is either one of the core `@context` or any other term, and a new core
/// datatype is a new variant of [`CoreDatatype`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, derive_more::From, derive_more::IsVariant)]
pub enum Datatype {
    /// A datatype of the core `@context` (clause C.6).
    #[display("{_0}")]
    Core(CoreDatatype),
    /// Any other datatype: a term, compact IRI or IRI.
    #[display("{_0}")]
    #[from(skip)]
    Other(VocabTerm),
}

// No derive: a term that names a core datatype is that datatype, never `Other`.
impl From<VocabTerm> for Datatype {
    fn from(term: VocabTerm) -> Self {
        term.as_ref().parse::<CoreDatatype>().map_or(Self::Other(term), Self::Core)
    }
}
