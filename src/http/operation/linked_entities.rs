//! Linked entity retrieval (NGSI-LD clause 4.5.23): the `join`, `joinLevel` and `containedBy` parameters of
//! Retrieve Entity and Query Entities (Tables 6.4.3.2-1 and 6.5.3.1-1).

use crate::{
    http::{QueryParameter, query_string::QueryString},
    model::EntityId,
    query::Join,
    snapshot::query_document::writer::QueryMembers,
};
use std::num::NonZeroU32;

/// The joins that keep the answer of Retrieve Entity one entity: `inline` and `@none`.
///
/// NGSI-LD clause 4.5.23.
///
/// A flattened join answers with an array of entities instead (clause 4.5.23.3), so it is asked for through
/// [`Flattened`], which changes the type of the answer.
///
/// ```
/// use ngsi_ld::{http::operation::InlineJoin, query::Join};
///
/// assert_eq!(Join::from(InlineJoin::Inline), Join::Inline);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InlineJoin {
    /// `inline`.
    Inline,
    /// `@none`.
    None,
}

/// Every inline join is a join.
// No derive maps the variants of one enum onto another.
impl From<InlineJoin> for Join {
    fn from(join: InlineJoin) -> Self {
        match join {
            InlineJoin::Inline => Self::Inline,
            InlineJoin::None => Self::None,
        }
    }
}

/// Asks Retrieve Entity for a flattened join.
///
/// The answer is the target entity and the entities its relationships target, as an array (`join=flat`, NGSI-LD clause
/// 4.5.23.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Flattened;

/// The linked entity retrieval of one request.
///
/// Tables 6.4.3.2-1, 6.5.3.1-1 and 5.2.23-1 say `joinLevel` is "only applicable if join parameter is present" (or is
/// `flat` or `inline`) and `containedBy` "only applicable if joinLevel is present": both describe what a broker does
/// with the parameter rather than forbid it, so neither is refused without the other, and the broker ignores it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LinkedEntities<'a> {
    /// `join`.
    pub(crate) join: Option<Join>,
    /// `joinLevel`.
    pub(crate) level: Option<NonZeroU32>,
    /// `containedBy`.
    pub(crate) contained_by: &'a [EntityId],
}

impl<'a> LinkedEntities<'a> {
    /// Adds the parameters to a query string.
    pub(crate) fn push_query(&self, query: &mut QueryString) {
        query.push_some(QueryParameter::Join, self.join);
        query.push_some(QueryParameter::JoinLevel, self.level);
        query.push_list(QueryParameter::ContainedBy, self.contained_by);
    }

    /// Fills the members of a `Query` body (clause 5.2.23) the retrieval holds: `join`, `joinLevel` and
    /// `containedBy`.
    pub(crate) fn query_members(&self, members: QueryMembers<'a>) -> QueryMembers<'a> {
        QueryMembers {
            join: self.join,
            join_level: self.level,
            contained_by: self.contained_by,
            ..members
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        http::{operation::linked_entities::LinkedEntities, query_string::QueryString},
        query::Join,
    };
    use std::num::NonZeroU32;

    #[test]
    fn a_join_level_without_a_linking_join_is_sent() {
        for join in [None, Some(Join::None)] {
            let linked = LinkedEntities {
                join,
                level: NonZeroU32::new(2),
                contained_by: &[],
            };
            let mut query = QueryString::default();
            linked.push_query(&mut query);
            assert!(query.to_string().ends_with("joinLevel=2"));
        }
    }

    #[test]
    fn contained_by_without_a_join_level_is_sent() {
        let ids = ["urn:ngsi-ld:Building:1".parse().unwrap()];
        for join in [None, Some(Join::Flat)] {
            let linked = LinkedEntities {
                join,
                level: None,
                contained_by: &ids,
            };
            let mut query = QueryString::default();
            linked.push_query(&mut query);
            assert!(query.to_string().ends_with("containedBy=urn%3Angsi-ld%3ABuilding%3A1"));
        }
    }

    #[test]
    fn the_parameters_print_their_spec_tokens() {
        let ids = ["urn:ngsi-ld:Building:1".parse().unwrap()];
        let linked = LinkedEntities {
            join: Some(Join::None),
            level: NonZeroU32::new(3),
            contained_by: &ids,
        };
        let mut query = QueryString::default();
        linked.push_query(&mut query);
        assert_eq!(query.to_string(), "join=%40none&joinLevel=3&containedBy=urn%3Angsi-ld%3ABuilding%3A1");
    }
}
