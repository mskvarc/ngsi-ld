//! Context Source Properties (NGSI-LD clause 4.9): the members a context source registration carries beyond Tables
//! 5.2.9-1 and 5.2.9-2, which describe the context source and which a context source filter (`csf`) targets.

use crate::{
    model::{AttributeName, NameError, draft::Draft},
    registration::{Problem, RegistrationMember, RegistrationOutputMember, member::RegistrationMandatoryMember},
    seal::Seal,
};
use indexmap::IndexMap;
use std::str::FromStr;

/// Why text cannot name a Context Source Property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum ContextSourcePropertyNameError {
    /// The text is not an attribute name (clause 4.6.2), or is one the core `@context` reserves.
    #[error("a context source property is named as an attribute is")]
    Name {
        /// Why.
        #[from]
        source: NameError,
    },
    /// The text names a member Tables 5.2.9-1 and 5.2.9-2 define for every registration.
    #[error("a context source property cannot be named after a member of the registration data type")]
    ReservedByRegistration,
}

/// Whether `name` is a member Tables 5.2.9-1, 5.2.9-2 or 5.2.2-1 define for a registration.
fn is_registration_member(name: &str) -> bool {
    name.parse::<RegistrationMandatoryMember>().is_ok() || name.parse::<RegistrationMember>().is_ok() || name.parse::<RegistrationOutputMember>().is_ok()
}

/// The name of a Context Source Property, which describes a context source.
///
/// A Context Source Property (NGSI-LD clause 4.9) is a member of a context source registration that describes the
/// context source, "defined when Context Sources are registered", and the target of a context source filter (`csf`).
///
/// Clause 5.2.9 names no such member, so a registration may carry any number of them beside the members of Tables
/// 5.2.9-1 and 5.2.9-2. The name is an attribute name (clause 4.6.2, the `AttrName` a `csf` query targets), so the
/// names the core `@context` reserves are refused, and so are the members of Tables 5.2.9-1, 5.2.9-2 and 5.2.2-1,
/// which would be read as those members; `@context` is one of the reserved names. Being an attribute name, it is
/// expanded and compacted as one under a `@context` (clause 5.5.7).
///
/// ```
/// use ngsi_ld::registration::ContextSourcePropertyName;
///
/// let name: ContextSourcePropertyName = "csourceProperty1".parse()?;
/// assert_eq!(name.to_string(), "csourceProperty1");
/// assert!("endpoint".parse::<ContextSourcePropertyName>().is_err());
/// assert!("timesSent".parse::<ContextSourcePropertyName>().is_err());
/// assert!("@context".parse::<ContextSourcePropertyName>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, derive_more::Display, derive_more::AsRef)]
#[display("{_0}")]
#[as_ref(forward)]
pub struct ContextSourcePropertyName(AttributeName);

impl ContextSourcePropertyName {
    /// The property named `name`, or `name` back when a member of the registration data type has it.
    pub(crate) fn from_attribute(name: AttributeName) -> Result<ContextSourcePropertyName, AttributeName> {
        if is_registration_member(name.as_ref()) {
            Err(name)
        } else {
            Ok(ContextSourcePropertyName(name))
        }
    }

    /// Converts the property name into the attribute name it is.
    #[cfg(feature = "linked-data")]
    pub(crate) fn into_attribute(self) -> AttributeName {
        self.0
    }

    /// Returns the name as an attribute name, the form a `@context` expands and compacts.
    #[must_use]
    pub const fn as_attribute(&self) -> &AttributeName {
        &self.0
    }
}

// No derive: the attribute name is checked against the members of the registration data type.
impl TryFrom<AttributeName> for ContextSourcePropertyName {
    type Error = ContextSourcePropertyNameError;

    fn try_from(name: AttributeName) -> Result<Self, Self::Error> {
        Self::from_attribute(name).map_err(|_| ContextSourcePropertyNameError::ReservedByRegistration)
    }
}

// No derive: the text is read as an attribute name, then checked against the members of the registration data
// type.
impl FromStr for ContextSourcePropertyName {
    type Err = ContextSourcePropertyNameError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::try_from(text.parse::<AttributeName>()?)
    }
}

draft!(
    /// A Context Source Property name given to a builder.
    ///
    /// It is a [`ContextSourcePropertyName`], or text validated when the builder finishes, so every invalid name is
    /// reported together.
    ///
    /// ```
    /// use ngsi_ld::registration::ContextSourcePropertyNameDraft;
    ///
    /// let name = ContextSourcePropertyNameDraft::from("csourceProperty1");
    /// # let _ = name;
    /// ```
    ContextSourcePropertyNameDraft(ContextSourcePropertyName, ContextSourcePropertyNameError)
);

/// Resolves the Context Source Properties given to a builder, in order, recording each invalid name with its
/// position and each name given twice.
pub(crate) fn resolve<T>(drafts: Vec<(ContextSourcePropertyNameDraft, T)>, problems: &mut Vec<Problem>) -> IndexMap<ContextSourcePropertyName, T> {
    let mut properties = IndexMap::with_capacity(drafts.len());
    for (index, (name, value)) in drafts.into_iter().enumerate() {
        match name.into_result(Seal::new()) {
            Err(source) => problems.push(Problem::ContextSourceProperty { index, source }),
            Ok(name) if properties.contains_key(&name) => problems.push(Problem::DuplicateContextSourceProperty { name }),
            Ok(name) => {
                properties.insert(name, value);
            }
        }
    }
    properties
}
