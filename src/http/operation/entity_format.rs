//! How an operation that returns entities asks for them and reads them: in a lossless representation, or in the
//! simplified one.

use crate::{
    http::{
        BodyError,
        Page,
        ResponseContext,
        ResponseError,
        operation::{
            ArgumentError,
            RepresentationSpelling,
            projection_rule::{ProjectionRequest, ProjectionRule},
        },
        query_string::QueryString,
        response::{Success, read_array},
    },
    model::{
        Completeness,
        Context,
        DocumentError,
        Entity,
        Lossless,
        NgsiLdMediaType,
        Output,
        Representation,
        SimplifiedEntity,
        representation::read::read_entity,
    },
};
use bytes::Bytes;
use http::{Response, StatusCode};
use jstrict::Value as Json;

/// Asks for entities in simplified representation.
///
/// Operations that return entities take either a [`Lossless`] representation, answered with [`Entity`] values, or this
/// marker, answered with [`SimplifiedEntity`] values (NGSI-LD clause 4.5.4), so the type of the answer follows the
/// format asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Simplified;

/// How an operation that returns entities asks for them and reads them; `C` is what the entities hold of their
/// `id` and `type`.
pub(crate) trait EntityFormat<C: Completeness>: Copy {
    /// What one entity of the answer is.
    type Item;

    /// The representation asked for, the token of the `format` parameter (clause 6.3.7).
    fn representation(self) -> Representation;

    /// The media type asked for in `Accept` (clauses 6.3.4 and 6.3.15).
    fn media_type(self) -> NgsiLdMediaType;

    /// Adds the parameters that ask for this format: `format` as `spelling` writes it (clause 6.3.7), and whatever
    /// else the format needs. The `options` parameter, which also carries `sysAttrs`, is the operation's to add.
    fn push_query(self, query: &mut QueryString, spelling: RepresentationSpelling) {
        spelling.push_format(query, self.representation());
    }

    /// Refuses a projection whose answer this format cannot read.
    fn check_projection(self, projection: ProjectionRequest<'_>) -> Result<(), ArgumentError>;

    /// Reads one entity.
    fn read(self, json: Json) -> Result<Self::Item, DocumentError>;

    /// Reads a list of entities, and the `@context` the list carries as a whole, if any; an error names the index
    /// of the entity at fault.
    fn read_list(self, json: Json) -> Result<(Vec<Self::Item>, Option<Context>), BodyError> {
        Ok((read_array(json, |item| self.read(item), BodyError::Document)?, None))
    }
}

impl<C: ProjectionRule> EntityFormat<C> for Lossless {
    type Item = Entity<Output, C>;

    fn representation(self) -> Representation {
        match self {
            Self::Normalized => Representation::Normalized,
            Self::Concise => Representation::Concise,
        }
    }

    fn media_type(self) -> NgsiLdMediaType {
        NgsiLdMediaType::Json
    }

    fn check_projection(self, projection: ProjectionRequest<'_>) -> Result<(), ArgumentError> {
        C::check(projection)
    }

    fn read(self, json: Json) -> Result<Entity<Output, C>, DocumentError> {
        read_entity(json, self)
    }
}

impl<C: ProjectionRule> EntityFormat<C> for Simplified {
    type Item = SimplifiedEntity<C>;

    fn representation(self) -> Representation {
        Representation::Simplified
    }

    fn media_type(self) -> NgsiLdMediaType {
        NgsiLdMediaType::Json
    }

    fn check_projection(self, projection: ProjectionRequest<'_>) -> Result<(), ArgumentError> {
        C::check(projection)
    }

    fn read(self, json: Json) -> Result<SimplifiedEntity<C>, DocumentError> {
        SimplifiedEntity::from_json(json)
    }
}

/// Reads a body that is one entity.
pub(crate) fn read_one<C: Completeness, F: EntityFormat<C>>(response: &Success, format: F) -> Result<F::Item, ResponseError> {
    format.read(response.json()?).map_err(|source| response.body_error(source))
}

/// Reads a body that is a list of entities, without the `@context` of the list as a whole.
pub(crate) fn read_many<C: Completeness, F: EntityFormat<C>>(response: &Success, format: F) -> Result<Vec<F::Item>, ResponseError> {
    read_list(response, format).map(|(items, _)| items)
}

/// Reads a body that is a list of entities, and the `@context` of the list as a whole.
fn read_list<C: Completeness, F: EntityFormat<C>>(response: &Success, format: F) -> Result<(Vec<F::Item>, Option<Context>), ResponseError> {
    format.read_list(response.json()?).map_err(|source| response.body_error(source))
}

/// Reads a page: 200, 201 when an entity map was created, or 203 when the broker amended the entities to another
/// version (clause 4.3.6.8).
pub(crate) fn decode_page<C: Completeness, F: EntityFormat<C>>(
    context: &ResponseContext,
    response: Response<Bytes>,
    format: F,
) -> Result<Page<F::Item>, ResponseError> {
    Page::decode(
        context,
        response,
        &[StatusCode::OK, StatusCode::CREATED, StatusCode::NON_AUTHORITATIVE_INFORMATION],
        |response| read_list(response, format),
    )
}
