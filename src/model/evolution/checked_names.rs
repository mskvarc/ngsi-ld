//! Checking the attributes a builder of a temporal representation is given by name (NGSI-LD clauses 4.5.9 and
//! 4.5.19): each name valid and given once, and each name clause 5.2.4 reserves for geometries holding a
//! `GeoProperty`'s evolution.

use crate::model::{
    AttributeKind,
    AttributeName,
    AttributeNameDraft,
    AttributesError,
    BuildProblem,
    ProblemKind,
    attribute::attributes::holds_geometries,
    draft::Draft,
};
use indexmap::IndexMap;

/// The attributes, each name valid and given once and each geometry name (`location`, `observationSpace`,
/// `operationSpace`) holding an attribute `kind` says is a `GeoProperty` (clause 5.2.4); every problem is recorded
/// and the attribute at fault left out.
pub(crate) fn checked_attributes<T>(
    attributes: Vec<(AttributeNameDraft, T)>,
    kind: impl Fn(&T) -> AttributeKind,
    problems: &mut Vec<BuildProblem>,
) -> IndexMap<AttributeName, T> {
    let mut checked = IndexMap::new();
    for (index, (name, attribute)) in attributes.into_iter().enumerate() {
        let name = match name.into_result() {
            Ok(name) => name,
            Err(source) => {
                problems.push(BuildProblem::top(ProblemKind::AttributeName { index, source }));
                continue;
            }
        };
        let refusal = if checked.contains_key(&name) {
            Some(AttributesError::ValuesNamedTwice)
        } else if holds_geometries(&name) && kind(&attribute) != AttributeKind::GeoProperty {
            Some(AttributesError::GeoPropertyRequired)
        } else {
            None
        };
        if let Some(source) = refusal {
            problems.push(BuildProblem::top(ProblemKind::Attributes { name, source }));
            continue;
        }
        checked.insert(name, attribute);
    }
    checked
}
