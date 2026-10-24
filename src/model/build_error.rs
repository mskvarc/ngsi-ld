//! Every problem a builder found, reported together.

use crate::model::{AttributeName, BuildProblem};
use std::{error::Error, fmt};

/// Every problem a builder found, in the order the builder calls were made; `P` is the problem of the document
/// family: [`BuildProblem`] for the data model, and the `Problem` of the query languages, subscriptions,
/// registrations and snapshots for theirs.
///
/// Builders accept convenient inputs (`&str` names, string values) and validate them once, when the document
/// is built, so a caller sees every problem at the same time rather than one per attempt. A value the caller
/// validates by hand (a name, a scope, an identifier, a language map, a geometry, raw JSON) converts its error into
/// a `BuildError` with `?`, so building a data-model document has one error type.
///
/// It displays as the number of problems and every problem, each followed by its chain of causes.
///
/// ```
/// use ngsi_ld::model::Entity;
///
/// let invalid = Entity::builder()
///     .id("urn:ngsi-ld:Vehicle:A 4567")
///     .entity_type("Vehicle")
///     .property("location", "Grand Place")
///     .build();
/// let error = invalid.err().map(|error| error.to_string());
/// assert_eq!(
///     error.as_deref(),
///     Some(
///         "2 problem(s): invalid identifier: not a valid URI or IRI; \
///          `location` cannot be added: the attribute must be a GeoProperty"
///     )
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, thiserror::Error)]
#[error("{} problem(s): {}", .problems.len(), ProblemList(&self.problems))]
pub struct BuildError<P: Error = BuildProblem> {
    /// The problems, in the order they were found; never empty.
    problems: Vec<P>,
}

impl<P: Error> BuildError<P> {
    /// Returns the problems, in the order they were found. There is at least one.
    #[must_use]
    pub fn problems(&self) -> &[P] {
        &self.problems
    }

    /// A build error holding `problems`, which are not empty.
    pub(crate) const fn new(problems: Vec<P>) -> Self {
        Self { problems }
    }

    /// A build error holding one problem.
    pub(crate) fn single(problem: P) -> Self {
        Self { problems: vec![problem] }
    }

    /// Returns `value` when no problem was found, and every problem otherwise.
    pub(crate) fn finish<T>(problems: Vec<P>, value: T) -> Result<T, Self> {
        if problems.is_empty() { Ok(value) } else { Err(Self { problems }) }
    }

    /// Returns `value`, or the problems found while building it; `value` is `None` only when a problem prevented it,
    /// and that problem was recorded.
    pub(crate) fn finish_partial<T>(problems: Vec<P>, value: Option<T>) -> Result<T, Self> {
        match value {
            Some(value) if problems.is_empty() => Ok(value),
            Some(_) | None => Err(Self { problems }),
        }
    }
}

impl BuildError {
    /// Moves the problems under the attribute `name`.
    pub(crate) fn within(self, name: &AttributeName) -> Vec<BuildProblem> {
        self.problems.into_iter().map(|problem| problem.nested_in(name)).collect()
    }
}

/// Renders a list of problems on one line: each problem followed by its causes, each after `: `, and the problems
/// separated by `; `.
struct ProblemList<'a, P>(&'a [P]);

// Joining a list has no derive.
impl<P: Error> fmt::Display for ProblemList<'_, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, problem) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str("; ")?;
            }
            write!(f, "{problem}")?;
            let mut cause = problem.source();
            while let Some(source) = cause {
                write!(f, ": {source}")?;
                cause = source.source();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
