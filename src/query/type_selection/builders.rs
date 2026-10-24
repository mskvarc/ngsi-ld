//! The builder of a [`TypeSelection`].
//!
//! [`TypeSelectionBuilder`].

use crate::{
    model::BuildError,
    query::{
        Problem,
        build_error::Deferred,
        type_selection::{SelectableType, TypeSelection, entity_type, parse_conjunction, parse_type},
    },
};

/// A type selection under construction, started by [`entity_type`] or [`all_of`](crate::query::type_selection::all_of).
///
/// Names are validated on the spot, without panicking; [`build`](Self::build) reports every invalid one.
///
/// ```
/// use ngsi_ld::query::type_selection;
///
/// let selection = type_selection::entity_type("Building")
///     .or_all_of(["Home", "Vehicle"])
///     .build()?;
/// assert_eq!(selection.to_string(), "Building|(Home;Vehicle)");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeSelectionBuilder(pub(super) Deferred<Vec<Vec<SelectableType>>>);

impl TypeSelectionBuilder {
    /// Adds the alternative of the entity type `name`.
    #[must_use]
    pub fn or(self, name: impl AsRef<str>) -> Self {
        Self(self.0.zip(parse_type(name.as_ref())).map(|(mut alternatives, selected)| {
            alternatives.push(vec![selected]);
            alternatives
        }))
    }

    /// Adds the alternative of the entities having all the types `names`.
    #[must_use]
    pub fn or_all_of(self, names: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        Self(self.0.zip(parse_conjunction(names)).map(|(mut alternatives, conjunction)| {
            alternatives.push(conjunction);
            alternatives
        }))
    }

    /// Returns the selection.
    ///
    /// # Errors
    ///
    /// Returns a [`BuildError`] listing every invalid entity type and every empty conjunction.
    pub fn build(self) -> Result<TypeSelection, BuildError<Problem>> {
        self.0.finish().map(|alternatives| TypeSelection { alternatives })
    }
}

/// A selection of the one entity type `name`, as [`entity_type`] starts it.
impl From<&str> for TypeSelectionBuilder {
    fn from(name: &str) -> Self {
        entity_type(name)
    }
}

/// A selection already built.
impl From<TypeSelection> for TypeSelectionBuilder {
    fn from(selection: TypeSelection) -> Self {
        Self(Deferred::valid(selection.alternatives))
    }
}

impl TryFrom<TypeSelectionBuilder> for TypeSelection {
    type Error = BuildError<Problem>;

    fn try_from(builder: TypeSelectionBuilder) -> Result<Self, Self::Error> {
        builder.build()
    }
}
