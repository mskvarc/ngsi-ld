//! The JSON form of an entity ordering: the `OrderingParams` object of NGSI-LD clause 5.2.43, which the `Query`
//! data type (Table 5.2.23-1) carries as its `ordering` member.

use crate::{
    model::{
        BuildError,
        DocumentError,
        MemberError,
        RawJson,
        geometry::Shape,
        reader::{array, object, once, parse_json, required, string, unexpected, within},
    },
    query::{
        ParseError,
        Problem,
        ordering::{EntityOrdering, OrderBy, builders::EntityOrderingBuilder},
    },
};
use jstrict::{Object, Value as Json, object::Entry};

/// A member of the `OrderingParams` object (Table 5.2.43-1).
///
/// The enum is `#[non_exhaustive]`: later versions of the specification may add members to the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr)]
#[non_exhaustive]
pub enum OrderingParamsMember {
    /// `orderBy`.
    #[strum(serialize = "orderBy")]
    OrderBy,
    /// `collation`.
    #[strum(serialize = "collation")]
    Collation,
    /// `geometry`.
    #[strum(serialize = "geometry")]
    Geometry,
    /// `coordinates`.
    #[strum(serialize = "coordinates")]
    Coordinates,
}

/// Why JSON data is not an `OrderingParams` object (NGSI-LD clause 5.2.43); a reader reports it as a
/// [`DocumentError<OrderingParamsError>`](DocumentError), whose `path` names the member at fault.
// `ParseError` carries no `Clone` or equality, so neither can the error that chains it.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OrderingParamsError {
    /// A problem every reader can meet: the text is not JSON, the data is not an object, a member is missing
    /// (`orderBy`, or `coordinates` beside a `geometry`), repeated, unknown or of the wrong JSON type, or
    /// `geometry` and `coordinates` do not make a geometry NGSI-LD accepts (clause 4.7.1).
    #[error(transparent)]
    Member(#[from] MemberError),
    /// The terms of `orderBy` do not follow the ordering language of clause 4.23.
    #[error("invalid orderBy")]
    OrderBy {
        /// Why.
        #[source]
        source: ParseError,
    },
    /// The members break a rule of the ordering: an invalid collation, an entity member sorted by distance, or a
    /// sort by distance without coordinates (clauses 4.23.1 and 4.23.3).
    #[error("invalid ordering")]
    Ordering {
        /// Every rule broken.
        #[source]
        source: BuildError<Problem>,
    },
}

/// Reads `orderBy`: an array whose strings, each a term or several separated by `,`, together are one ordering of
/// the language of clause 4.23.
fn read_order_by(json: Json) -> Result<OrderBy, DocumentError<OrderingParamsError>> {
    array(json)?
        .into_iter()
        .enumerate()
        .map(|(index, term)| string(term, OrderingParamsMember::OrderBy.into()).map_err(|error| error.at_index(index)))
        .collect::<Result<Vec<_>, _>>()?
        .join(",")
        .parse()
        .map_err(|source| DocumentError::top(OrderingParamsError::OrderBy { source }))
}

/// Reads the origin: `coordinates` of the `geometry` named, a Point when none is (Table 5.2.43-1).
fn origin(geometry_type: Option<String>, coordinates: Option<Json>) -> Result<Option<Shape>, DocumentError<OrderingParamsError>> {
    match (coordinates, geometry_type) {
        (Some(coordinates), geometry_type) => Shape::from_parts(geometry_type.as_deref().unwrap_or("Point"), &coordinates)
            .map(Some)
            .map_err(|source| DocumentError::top(MemberError::Geometry { source }.into())),
        (None, Some(_)) => Err(DocumentError::top(
            MemberError::MissingMember {
                member: OrderingParamsMember::Coordinates.into(),
            }
            .into(),
        )),
        (None, None) => Ok(None),
    }
}

impl EntityOrdering {
    /// Reads the text of an `OrderingParams` object (NGSI-LD clause 5.2.43), as a `Query` body carries it in its
    /// `ordering` member (Table 5.2.23-1).
    ///
    /// ```
    /// use ngsi_ld::query::ordering::EntityOrdering;
    ///
    /// let nearest = EntityOrdering::parse(
    ///     r#"{"orderBy": ["location;dist-asc", "name"], "coordinates": [8, 40]}"#,
    /// )?;
    /// assert_eq!(nearest.order_by().to_string(), "location;dist-asc,name");
    /// assert_eq!(
    ///     nearest.render().to_string(),
    ///     r#"{"orderBy":["location;dist-asc","name"],"coordinates":[8,40]}"#
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError<OrderingParamsError>`](DocumentError) naming the member at fault and what is
    /// missing or invalid.
    pub fn parse(text: &str) -> Result<Self, DocumentError<OrderingParamsError>> {
        Self::from_json(parse_json(text)?)
    }

    /// Writes the `OrderingParams` object in canonical form: `orderBy` with one element per term, `collation`,
    /// then the origin as `geometry`, left out for a Point, and `coordinates`.
    #[must_use]
    pub fn render(&self) -> RawJson {
        RawJson::from_json(self.to_json())
    }

    /// Reads an `OrderingParams` object (Table 5.2.43-1).
    pub(crate) fn from_json(json: Json) -> Result<Self, DocumentError<OrderingParamsError>> {
        let mut order_by = None;
        let mut collation = None;
        let mut geometry_type = None;
        let mut coordinates = None;
        for Entry { key, value } in object(json)? {
            let key = key.as_str();
            let Ok(member) = key.parse::<OrderingParamsMember>() else {
                return Err(unexpected(key));
            };
            let key = member.into();
            match member {
                OrderingParamsMember::OrderBy => once(&mut order_by, key, within(key, read_order_by(value))?)?,
                OrderingParamsMember::Collation => once(&mut collation, key, within(key, string(value, key))?)?,
                OrderingParamsMember::Geometry => once(&mut geometry_type, key, within(key, string(value, key))?)?,
                OrderingParamsMember::Coordinates => once(&mut coordinates, key, value)?,
            }
        }
        let builder = EntityOrderingBuilder::from(required(order_by, OrderingParamsMember::OrderBy.into())?);
        let builder = match collation {
            Some(collation) => builder.collation(collation),
            None => builder,
        };
        let builder = match origin(geometry_type, coordinates)? {
            Some(origin) => builder.origin(origin),
            None => builder,
        };
        builder.build().map_err(|source| DocumentError::top(OrderingParamsError::Ordering { source }))
    }

    /// Writes the `OrderingParams` object: `orderBy` with one element per term, `collation`, and the origin as
    /// `geometry`, left out for a Point, the default of Table 5.2.43-1, and `coordinates`.
    pub(crate) fn to_json(&self) -> Json {
        let mut object = Object::new();
        let terms = self.order_by().terms().iter().map(|term| Json::from(term.to_string())).collect();
        object.push(<&str>::from(OrderingParamsMember::OrderBy).into(), Json::Array(terms));
        if let Some(collation) = self.collation() {
            object.push(<&str>::from(OrderingParamsMember::Collation).into(), Json::from(collation.as_str()));
        }
        if let Some(origin) = self.origin() {
            if !origin.is_point() {
                object.push(<&str>::from(OrderingParamsMember::Geometry).into(), Json::from(origin.type_name()));
            }
            object.push(<&str>::from(OrderingParamsMember::Coordinates).into(), origin.coordinates_json());
        }
        Json::Object(object)
    }
}

#[cfg(test)]
mod tests;
