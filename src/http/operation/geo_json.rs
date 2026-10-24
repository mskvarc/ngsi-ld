//! Asking for entities in `GeoJSON` representation (NGSI-LD clauses 4.5.16, 4.5.17 and 6.3.15).

use crate::{
    http::{
        BodyError,
        QueryParameter,
        operation::{ArgumentError, RepresentationSpelling, Simplified, entity_format::EntityFormat, projection_rule::ProjectionRequest},
        query_string::QueryString,
    },
    model::{
        AttributeName,
        Complete,
        Context,
        DocumentError,
        Entity,
        Feature,
        Lossless,
        NgsiLdMediaType,
        Output,
        Representation,
        SimplifiedEntity,
        feature::{read_collection, read_feature},
        representation::read::read_entity,
    },
};
use jstrict::Value as Json;

/// Asks for entities in `GeoJSON` representation (`Accept: application/geo+json`, clause 6.3.15): one entity is
/// answered with a `GeoJSON` Feature, a list of entities with a `FeatureCollection` (clauses 4.5.16 and 4.5.17).
///
/// `F` is the representation of each entity within its Feature: a [`Lossless`] one answers with
/// [`Feature<Entity<Output>>`] values, [`Simplified`] with [`Feature<SimplifiedEntity>`] values. The
/// `geometryProperty` parameter, which clauses 5.7.1.4 and 5.7.2.4 refuse without the `GeoJSON` `Accept` header,
/// exists only here.
///
/// A projection never removes the `id` and the `type` of a Feature: Tables 5.2.29-1 and 5.2.31-1 make both
/// mandatory, and clauses 5.7.1.5 and 5.7.2.5 let `pick` and `omit` reduce only its attributes.
///
/// ```
/// use ngsi_ld::{
///     http::operation::{GeoJson, Simplified},
///     model::AttributeName,
/// };
///
/// let site: AttributeName = "site".parse()?;
/// let format = GeoJson {
///     form: Simplified,
///     geometry_property: Some(&site),
/// };
/// # let _ = format;
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct GeoJson<'a, F> {
    /// The representation of each entity within its Feature.
    pub form: F,
    /// The `GeoProperty` whose value is the `geometry` of each Feature (`geometryProperty`); `None` selects
    /// `location` (clause 4.5.16.1).
    pub geometry_property: Option<&'a AttributeName>,
}

/// The parameters of a `GeoJSON` request: the `format` of the entities within their Features, as `spelling`
/// writes it, then `geometryProperty`.
fn push_geo_json_query(representation: Representation, spelling: RepresentationSpelling, geometry_property: Option<&AttributeName>, query: &mut QueryString) {
    spelling.push_format(query, representation);
    query.push_some(QueryParameter::GeometryProperty, geometry_property);
}

impl EntityFormat<Complete> for GeoJson<'_, Lossless> {
    type Item = Feature<Entity<Output>>;

    fn representation(self) -> Representation {
        EntityFormat::<Complete>::representation(self.form)
    }

    fn media_type(self) -> NgsiLdMediaType {
        NgsiLdMediaType::GeoJson
    }

    fn push_query(self, query: &mut QueryString, spelling: RepresentationSpelling) {
        push_geo_json_query(EntityFormat::<Complete>::representation(self), spelling, self.geometry_property, query);
    }

    fn check_projection(self, _: ProjectionRequest<'_>) -> Result<(), ArgumentError> {
        Ok(())
    }

    fn read(self, json: Json) -> Result<Feature<Entity<Output>>, DocumentError> {
        read_feature(json, |entity| read_entity(entity, self.form))
    }

    fn read_list(self, json: Json) -> Result<(Vec<Feature<Entity<Output>>>, Option<Context>), BodyError> {
        let collection = read_collection(json, |entity| read_entity(entity, self.form))?;
        Ok((collection.features, collection.context))
    }
}

impl EntityFormat<Complete> for GeoJson<'_, Simplified> {
    type Item = Feature<SimplifiedEntity>;

    fn representation(self) -> Representation {
        Representation::Simplified
    }

    fn media_type(self) -> NgsiLdMediaType {
        NgsiLdMediaType::GeoJson
    }

    fn push_query(self, query: &mut QueryString, spelling: RepresentationSpelling) {
        push_geo_json_query(Representation::Simplified, spelling, self.geometry_property, query);
    }

    fn check_projection(self, _: ProjectionRequest<'_>) -> Result<(), ArgumentError> {
        Ok(())
    }

    fn read(self, json: Json) -> Result<Feature<SimplifiedEntity>, DocumentError> {
        read_feature(json, SimplifiedEntity::from_json)
    }

    fn read_list(self, json: Json) -> Result<(Vec<Feature<SimplifiedEntity>>, Option<Context>), BodyError> {
        let collection = read_collection(json, SimplifiedEntity::from_json)?;
        Ok((collection.features, collection.context))
    }
}
