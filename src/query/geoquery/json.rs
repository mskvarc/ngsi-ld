//! The JSON form of a geoquery: the `GeoQuery` object of NGSI-LD clause 5.2.13, which the `Query` body of Query
//! Entities (clause 5.2.23) and a subscription carry as their `geoQ` member.

use crate::{
    model::{
        DocumentError,
        MemberError,
        RawJson,
        geometry::Shape,
        reader::{object, once, parse_json, parsed, required, string, unexpected, within},
    },
    query::{
        ParseError,
        geoquery::{GeoQuery, GeoRelation},
    },
};
use jstrict::{Object, Value as Json, object::Entry};

/// A member of the `GeoQuery` object (Table 5.2.13-1).
///
/// The enum is `#[non_exhaustive]`: later versions of the specification may add members to the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr)]
#[non_exhaustive]
pub enum GeoQueryMember {
    /// `georel`.
    #[strum(serialize = "georel")]
    Georel,
    /// `geometry`.
    #[strum(serialize = "geometry")]
    Geometry,
    /// `coordinates`.
    #[strum(serialize = "coordinates")]
    Coordinates,
    /// `geoproperty`.
    #[strum(serialize = "geoproperty")]
    Geoproperty,
}

/// Why JSON data is not a `GeoQuery` object (NGSI-LD clause 5.2.13); a reader reports it as a
/// [`DocumentError<GeoQueryError>`](DocumentError), whose `path` names the member at fault.
// `ParseError` carries no `Clone` or equality, so neither can the error that chains it.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GeoQueryError {
    /// A problem every reader can meet: the text is not JSON, the data is not an object, a member is missing,
    /// repeated, unknown or not a string, `geoproperty` is not an attribute name, or `geometry` and `coordinates`
    /// do not make a geometry NGSI-LD accepts (clause 4.7.1).
    #[error(transparent)]
    Member(#[from] MemberError),
    /// `georel` is not a geo-relationship of clause 4.10.
    #[error("invalid geo-relationship")]
    Relation {
        /// Why the relationship is invalid.
        #[source]
        source: ParseError,
    },
    /// `coordinates` is a string that does not hold JSON (clause 4.7.2).
    #[error("coordinates written as a string must hold JSON")]
    CoordinatesNotJson,
}

/// Reads `georel`.
fn read_relation(json: Json) -> Result<GeoRelation, DocumentError<GeoQueryError>> {
    string(json, GeoQueryMember::Georel.into())?
        .parse()
        .map_err(|source| DocumentError::top(GeoQueryError::Relation { source }))
}

/// Reads `coordinates`: a JSON array or, for JSON-LD compatibility, a string holding one (clause 4.7.2).
fn read_coordinates(json: Json) -> Result<Json, DocumentError<GeoQueryError>> {
    match json {
        Json::String(encoded) => encoded.as_str().parse().map_err(|_| DocumentError::top(GeoQueryError::CoordinatesNotJson)),
        coordinates @ (Json::Null | Json::Boolean(_) | Json::Number(_) | Json::Array(_) | Json::Object(_)) => Ok(coordinates),
    }
}

impl GeoQuery {
    /// Reads the text of a `GeoQuery` object (NGSI-LD clause 5.2.13), as a subscription or a `Query` body carries it
    /// in its `geoQ` member. `coordinates` is a JSON array or, for JSON-LD compatibility, a string holding one
    /// (clauses 5.2.13 and 4.7.2).
    ///
    /// ```
    /// use ngsi_ld::query::geoquery::GeoQuery;
    ///
    /// let query =
    ///     GeoQuery::parse(r#"{"georel": "within", "geometry": "Point", "coordinates": "[8,40]"}"#)?;
    /// assert_eq!(
    ///     query.render().to_string(),
    ///     r#"{"georel":"within","geometry":"Point","coordinates":[8,40]}"#
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError<GeoQueryError>`](DocumentError) naming the member at fault and what is missing or
    /// invalid.
    pub fn parse(text: &str) -> Result<Self, DocumentError<GeoQueryError>> {
        Self::from_json(parse_json(text)?)
    }

    /// Writes the `GeoQuery` object in canonical form: `georel`, `geometry`, `coordinates` as a JSON array, and
    /// `geoproperty` when one is named, in the order Annex C.4 writes them.
    #[must_use]
    pub fn render(&self) -> RawJson {
        RawJson::from_json(self.to_json())
    }

    /// Reads a `GeoQuery` object (Table 5.2.13-1). `coordinates` is a JSON array or, for JSON-LD compatibility, a
    /// string holding one (clauses 5.2.13 and 4.7.2).
    pub(crate) fn from_json(json: Json) -> Result<Self, DocumentError<GeoQueryError>> {
        let mut relation = None;
        let mut geometry_type = None;
        let mut coordinates = None;
        let mut geoproperty = None;
        for Entry { key, value } in object(json)? {
            let key = key.as_str();
            let Ok(member) = key.parse::<GeoQueryMember>() else {
                return Err(unexpected(key));
            };
            let key = member.into();
            match member {
                GeoQueryMember::Georel => once(&mut relation, key, within(key, read_relation(value))?)?,
                GeoQueryMember::Geometry => once(&mut geometry_type, key, within(key, string(value, key))?)?,
                GeoQueryMember::Coordinates => once(&mut coordinates, key, within(key, read_coordinates(value))?)?,
                GeoQueryMember::Geoproperty => once(&mut geoproperty, key, within(key, parsed(value, key))?)?,
            }
        }
        let relation = required(relation, GeoQueryMember::Georel.into())?;
        let geometry_type = required(geometry_type, GeoQueryMember::Geometry.into())?;
        let coordinates = required(coordinates, GeoQueryMember::Coordinates.into())?;
        let geometry = Shape::from_parts(&geometry_type, &coordinates).map_err(|source| DocumentError::top(MemberError::Geometry { source }.into()))?;
        Ok(Self {
            relation,
            geometry,
            geoproperty,
        })
    }

    /// Writes the `GeoQuery` object: `georel`, `geometry`, `coordinates` as a JSON array, and `geoproperty` when
    /// one is named, in the order Annex C.4 writes them.
    pub(crate) fn to_json(&self) -> Json {
        let mut object = Object::new();
        object.push(<&str>::from(GeoQueryMember::Georel).into(), Json::from(self.relation.to_string()));
        object.push(<&str>::from(GeoQueryMember::Geometry).into(), Json::from(self.geometry.type_name()));
        object.push(<&str>::from(GeoQueryMember::Coordinates).into(), self.geometry.coordinates_json());
        if let Some(geoproperty) = &self.geoproperty {
            object.push(<&str>::from(GeoQueryMember::Geoproperty).into(), Json::from(geoproperty.as_ref()));
        }
        Json::Object(object)
    }
}
