use crate::model::{
    NotFinite,
    Number,
    RawJson,
    geometry::{
        BoundingBox,
        Dimension,
        ForbiddenGeometryMember,
        ForeignMemberName,
        ForeignMemberNameError,
        Geometry,
        GeometryError,
        GeometryMember,
        LinearRing,
        Position,
        Shape,
    },
};
#[cfg(feature = "geo-types")]
use geo_types::{Geometry as GeoTypesGeometry, Rect, coord};
use jstrict::Value as Json;
use pretty_assertions::assert_eq;

fn parse(json: &str) -> Result<Geometry, GeometryError> {
    Geometry::from_json(&json.parse::<Json>().unwrap())
}

fn round_trips(json: &str) {
    assert_eq!(parse(json).unwrap().to_string(), json);
}

fn point(longitude: f64, latitude: f64) -> Position {
    Position::try_new(longitude, latitude).unwrap()
}

#[test]
fn a_point_is_accepted_and_written_back() {
    round_trips(r#"{"type":"Point","coordinates":[13.3986,52.5547]}"#);
}

#[test]
fn a_geometry_collection_is_rejected() {
    assert_eq!(
        parse(r#"{"type":"GeometryCollection","geometries":[]}"#),
        Err(GeometryError::GeometryCollection)
    );
    assert_eq!(
        parse(r#"{"type":"GeometryCollection","coordinates":[]}"#),
        Err(GeometryError::GeometryCollection)
    );
}

#[test]
fn a_polygon_whose_ring_is_not_closed_is_rejected() {
    assert_eq!(
        parse(r#"{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0,1]]]}"#),
        Err(GeometryError::RingNotClosed)
    );
}

#[test]
fn a_closed_polygon_is_accepted_and_a_ring_closed_with_a_different_spelling_counts_as_closed() {
    round_trips(r#"{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0.0,0]]]}"#);
    round_trips(r#"{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[-0.0,0]]]}"#);
}

#[test]
fn every_geometry_type_round_trips() {
    for json in [
        r#"{"type":"MultiPoint","coordinates":[[1,2],[3,4,5]]}"#,
        r#"{"type":"LineString","coordinates":[[1,2],[3,4]]}"#,
        r#"{"type":"MultiLineString","coordinates":[[[1,2],[3,4]],[[5,6],[7,8]]]}"#,
        r#"{"type":"MultiPolygon","coordinates":[[[[0,0],[1,0],[1,1],[0,0]]]]}"#,
    ] {
        round_trips(json);
    }
}

#[test]
fn positions_need_two_or_three_numbers() {
    assert_eq!(parse(r#"{"type":"Point","coordinates":[1]}"#), Err(GeometryError::InvalidPosition));
    assert_eq!(parse(r#"{"type":"Point","coordinates":[1,2,3,4]}"#), Err(GeometryError::InvalidPosition));
}

#[test]
fn a_position_read_outside_wgs_84_is_kept_as_written() {
    round_trips(r#"{"type":"Point","coordinates":[181,0]}"#);
    round_trips(r#"{"type":"Point","coordinates":[0,-91]}"#);
}

#[test]
fn a_coordinate_too_large_for_a_float_is_refused_on_read() {
    assert!(matches!(
        parse(r#"{"type":"Point","coordinates":[1e400,0]}"#),
        Err(GeometryError::InvalidCoordinate {
            source: NotFinite::PositiveInfinity
        })
    ));
}

#[test]
fn a_position_built_in_code_must_lie_within_wgs_84() {
    assert_eq!(
        Position::try_new(181.0, 0.0),
        Err(GeometryError::LongitudeOutOfRange { value: Number::from(181) })
    );
    assert_eq!(
        Position::try_new(0.0, -91.0),
        Err(GeometryError::LatitudeOutOfRange { value: Number::from(-91) })
    );
}

#[test]
fn a_line_string_needs_two_positions() {
    assert_eq!(
        parse(r#"{"type":"LineString","coordinates":[[1,2]]}"#),
        Err(GeometryError::TooFewPositions { minimum: 2, found: 1 })
    );
}

#[test]
fn a_geometry_written_as_a_string_is_read() {
    let encoded = Json::from(r#"{"type":"Point","coordinates":[8,40]}"#);
    assert_eq!(Geometry::from_json(&encoded).unwrap().to_string(), r#"{"type":"Point","coordinates":[8,40]}"#);
}

#[test]
fn members_are_written_in_the_canonical_order() {
    assert_eq!(
        parse(r#"{"coordinates":[8,40],"type":"Point"}"#).unwrap().to_string(),
        r#"{"type":"Point","coordinates":[8,40]}"#
    );
    assert_eq!(
        parse(r#"{"bbox":[8,40,8,40],"coordinates":[8,40],"type":"Point"}"#).unwrap().to_string(),
        r#"{"type":"Point","coordinates":[8,40],"bbox":[8,40,8,40]}"#
    );
}

#[test]
fn missing_repeated_and_foreign_members_are_reported() {
    assert_eq!(
        parse(r#"{"coordinates":[1,2]}"#),
        Err(GeometryError::MissingMember { member: GeometryMember::Type })
    );
    assert_eq!(
        parse(r#"{"type":"Point","type":"Point","coordinates":[1,2]}"#),
        Err(GeometryError::DuplicateMember { member: GeometryMember::Type })
    );
    assert_eq!(
        parse(r#"{"type":"Point","coordinates":[1,2],"bbox":[1,2,1,2],"bbox":[1,2,1,2]}"#),
        Err(GeometryError::DuplicateMember { member: GeometryMember::Bbox })
    );
    assert_eq!(parse(r#"{"type":"Circle","coordinates":[1,2]}"#), Err(GeometryError::UnknownType));
}

#[test]
fn foreign_members_round_trip_in_order_after_the_defined_members() {
    round_trips(r#"{"type":"Point","coordinates":[1.5,2.5],"plot name":"7"}"#);
    round_trips(
        r#"{"type":"Polygon","coordinates":[[[0,0],[4,0],[4,4],[0,0]]],"bbox":[0,0,4,4],"title":"parcel","extra":{"nested":{"deep":[1,null,1.10]},"coordinates":true},"crs":null}"#,
    );
    assert_eq!(
        parse(r#"{"plot name":"7","coordinates":[1.5,2.5],"type":"Point"}"#).unwrap().to_string(),
        r#"{"type":"Point","coordinates":[1.5,2.5],"plot name":"7"}"#
    );
}

#[test]
fn foreign_members_are_read_with_their_values() {
    let geometry = parse(r#"{"type":"Point","coordinates":[1,2],"crs":{"type":"name"},"count":3}"#).unwrap();
    let members: Vec<(&str, String)> = geometry
        .foreign_members()
        .iter()
        .map(|(name, value)| (name.as_ref(), value.to_string()))
        .collect();
    assert_eq!(members, [("crs", r#"{"type":"name"}"#.to_owned()), ("count", "3".to_owned())]);
    assert_eq!(geometry.into_shape().to_string(), r#"{"type":"Point","coordinates":[1,2]}"#);
}

#[test]
fn a_foreign_member_written_twice_is_refused() {
    assert_eq!(
        parse(r#"{"type":"Point","coordinates":[1,2],"crs":1,"crs":2}"#),
        Err(GeometryError::DuplicateForeignMember { name: "crs".parse().unwrap() })
    );
}

#[test]
fn members_rfc_7946_section_7_1_forbids_on_a_geometry_are_refused() {
    for (member, forbidden) in [
        ("geometry", ForbiddenGeometryMember::Geometry),
        ("properties", ForbiddenGeometryMember::Properties),
        ("features", ForbiddenGeometryMember::Features),
    ] {
        assert_eq!(
            parse(&format!(r#"{{"type":"Point","coordinates":[1,2],"{member}":{{}}}}"#)),
            Err(GeometryError::ForeignMember {
                source: ForeignMemberNameError::Forbidden { member: forbidden }
            })
        );
    }
}

#[test]
fn a_foreign_member_name_is_neither_a_defined_nor_a_forbidden_member() {
    for (name, member) in [
        ("type", GeometryMember::Type),
        ("coordinates", GeometryMember::Coordinates),
        ("bbox", GeometryMember::Bbox),
    ] {
        assert_eq!(name.parse::<ForeignMemberName>(), Err(ForeignMemberNameError::Defined { member }));
    }
    assert_eq!(
        "features".parse::<ForeignMemberName>(),
        Err(ForeignMemberNameError::Forbidden {
            member: ForbiddenGeometryMember::Features
        })
    );
    for name in ["plot name", "", "Type", "geometries", "@context"] {
        assert_eq!(name.parse::<ForeignMemberName>().unwrap().as_ref(), name);
    }
}

#[test]
fn a_foreign_member_set_in_code_is_written_and_replaced_in_place() {
    let geometry = parse(r#"{"type":"Point","coordinates":[1,2],"a":1,"b":2}"#)
        .unwrap()
        .with_foreign_member("a".parse().unwrap(), "[3]".parse::<RawJson>().unwrap())
        .with_foreign_member("c".parse().unwrap(), "{}".parse::<RawJson>().unwrap());
    assert_eq!(geometry.to_string(), r#"{"type":"Point","coordinates":[1,2],"a":[3],"b":2,"c":{}}"#);
}

#[test]
fn a_geometry_written_as_a_string_keeps_its_foreign_members() {
    let encoded = Json::from(r#"{"type":"Point","coordinates":[1,2],"note":"x"}"#);
    assert_eq!(
        Geometry::from_json(&encoded).unwrap().to_string(),
        r#"{"type":"Point","coordinates":[1,2],"note":"x"}"#
    );
}

#[test]
fn a_bounding_box_round_trips() {
    round_trips(r#"{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0,0]]],"bbox":[0,0,1,1]}"#);
    round_trips(r#"{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0,0]]],"bbox":[0.0,0,1.10,1]}"#);
}

#[test]
fn a_three_dimensional_bounding_box_keeps_its_altitudes() {
    let geometry = parse(r#"{"type":"Point","coordinates":[1,2,3],"bbox":[1,2,3,1,2,3]}"#).unwrap();
    assert_eq!(geometry.to_string(), r#"{"type":"Point","coordinates":[1,2,3],"bbox":[1,2,3,1,2,3]}"#);
    let bbox = geometry.bbox().unwrap();
    assert_eq!(bbox.south_west().altitude().map(ToString::to_string), Some("3".to_owned()));
    assert_eq!(bbox.dimension(), Dimension::Three);
}

#[test]
fn a_bounding_box_may_cross_the_antimeridian() {
    assert!(parse(r#"{"type":"LineString","coordinates":[[170,0],[-170,1]],"bbox":[170,0,-170,1]}"#).is_ok());
}

#[test]
fn a_bounding_box_must_be_four_or_six_numbers() {
    for bbox in [
        "[0,0,1]",
        "[0,0,1,1,2]",
        "[]",
        "[0,0,0,0,1,1,1,1]",
        r#"[0,"0",1,1]"#,
        r#""0,0,1,1""#,
        "{}",
        "null",
    ] {
        assert_eq!(
            parse(&format!(r#"{{"type":"Point","coordinates":[0,0],"bbox":{bbox}}}"#)),
            Err(GeometryError::InvalidBoundingBox),
            "{bbox}"
        );
    }
}

#[test]
fn a_bounding_box_must_have_the_dimension_of_its_geometry() {
    assert_eq!(
        parse(r#"{"type":"Point","coordinates":[1,2],"bbox":[1,2,0,1,2,0]}"#),
        Err(GeometryError::BoundingBoxDimension {
            geometry: Dimension::Two,
            bounding_box: Dimension::Three,
        })
    );
    assert_eq!(
        parse(r#"{"type":"Point","coordinates":[1,2,3],"bbox":[1,2,1,2]}"#),
        Err(GeometryError::BoundingBoxDimension {
            geometry: Dimension::Three,
            bounding_box: Dimension::Two,
        })
    );
}

#[test]
fn a_geometry_with_any_altitude_is_three_dimensional() {
    assert!(parse(r#"{"type":"MultiPoint","coordinates":[[1,2],[3,4,5]],"bbox":[1,2,0,3,4,5]}"#).is_ok());
    assert_eq!(
        parse(r#"{"type":"MultiPoint","coordinates":[[1,2],[3,4,5]],"bbox":[1,2,3,4]}"#),
        Err(GeometryError::BoundingBoxDimension {
            geometry: Dimension::Three,
            bounding_box: Dimension::Two,
        })
    );
}

#[test]
fn a_bounding_box_must_not_have_its_south_north_of_its_north() {
    assert_eq!(
        parse(r#"{"type":"Point","coordinates":[0,0],"bbox":[0,1,1,0]}"#),
        Err(GeometryError::BoundingBoxLatitudes {
            south: Number::from(1),
            north: Number::from(0)
        })
    );
}

#[test]
fn a_bounding_box_built_in_code_is_checked() {
    let three = Position::try_with_altitude(0.0, 0.0, 0.0).unwrap();
    assert_eq!(BoundingBox::try_new(point(0.0, 0.0), three), Err(GeometryError::BoundingBoxCorners));
    assert_eq!(
        BoundingBox::try_new(point(0.0, 1.0), point(1.0, 0.0)),
        Err(GeometryError::BoundingBoxLatitudes {
            south: Number::from(1),
            north: Number::from(0)
        })
    );
    let three_dimensional = BoundingBox::try_new(
        Position::try_with_altitude(0.0, 0.0, 0.0).unwrap(),
        Position::try_with_altitude(1.0, 1.0, 1.0).unwrap(),
    )
    .unwrap();
    assert_eq!(
        Geometry::from(Shape::Point(point(0.0, 0.0))).with_bbox(three_dimensional),
        Err(GeometryError::BoundingBoxDimension {
            geometry: Dimension::Two,
            bounding_box: Dimension::Three,
        })
    );
}

#[test]
fn a_ring_built_in_code_must_be_closed() {
    let positions = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].map(|(x, y)| point(x, y));
    assert_eq!(LinearRing::try_new(positions.to_vec()), Err(GeometryError::RingNotClosed));
    let closed_by_negative_zero = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (-0.0, 0.0)].map(|(x, y)| point(x, y));
    assert!(LinearRing::try_new(closed_by_negative_zero.to_vec()).is_ok());
}

#[cfg(feature = "geo-types")]
#[test]
fn geo_types_conversion_round_trips_and_rejects_shapes_geojson_lacks() {
    let polygon = parse(r#"{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0,0]]]}"#).unwrap().into_shape();
    let converted = GeoTypesGeometry::from(&polygon);
    assert_eq!(Shape::try_from(converted).unwrap(), polygon);
    let rect = GeoTypesGeometry::Rect(Rect::new(coord! { x: 0.0, y: 0.0 }, coord! { x: 1.0, y: 1.0 }));
    assert_eq!(Shape::try_from(rect), Err(GeometryError::UnknownType));
}

#[cfg(feature = "geojson")]
#[test]
fn geojson_conversion_keeps_the_altitude() {
    let point = parse(r#"{"type":"Point","coordinates":[1,2,3]}"#).unwrap();
    let converted = <geojson::Geometry>::try_from(&point).unwrap();
    assert_eq!(Geometry::try_from(&converted).unwrap(), point);
}

#[cfg(feature = "geojson")]
#[test]
fn geojson_conversion_keeps_the_foreign_members() {
    let point = parse(r#"{"type":"Point","coordinates":[1,2],"plot name":"7","extra":{"deep":[1,-2,0.5,true,null]},"big":18446744073709551615}"#).unwrap();
    let converted = <geojson::Geometry>::try_from(&point).unwrap();
    let members = converted.foreign_members.as_ref().unwrap();
    let mut names: Vec<&str> = members.keys().map(String::as_str).collect();
    names.sort_unstable();
    assert_eq!(names, ["big", "extra", "plot name"]);
    assert_eq!(Geometry::try_from(&converted).unwrap(), point);
}

#[cfg(feature = "geojson")]
#[test]
fn geojson_conversion_refuses_a_foreign_number_beyond_the_range_of_an_f64() {
    let point = parse(r#"{"type":"Point","coordinates":[1,2],"huge":1e400}"#).unwrap();
    assert_eq!(
        <geojson::Geometry>::try_from(&point),
        Err(GeometryError::ForeignMemberNumber { name: "huge".parse().unwrap() })
    );
}

#[cfg(feature = "geojson")]
#[test]
fn geojson_foreign_members_are_held_to_rfc_7946_section_7_1() {
    let mut converted = <geojson::Geometry>::try_from(&parse(r#"{"type":"Point","coordinates":[0,0]}"#).unwrap()).unwrap();
    converted.foreign_members = Some(geojson::JsonObject::from_iter([("properties".to_owned(), geojson::JsonValue::Null)]));
    assert_eq!(
        Geometry::try_from(&converted),
        Err(GeometryError::ForeignMember {
            source: ForeignMemberNameError::Forbidden {
                member: ForbiddenGeometryMember::Properties
            }
        })
    );
}

#[cfg(feature = "geojson")]
#[test]
fn geojson_conversion_keeps_the_bounding_box() {
    let polygon = parse(r#"{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0,0]]],"bbox":[0,0,1,1]}"#).unwrap();
    let converted = <geojson::Geometry>::try_from(&polygon).unwrap();
    assert_eq!(converted.bbox, Some(vec![0.0, 0.0, 1.0, 1.0]));
    assert_eq!(Geometry::try_from(&converted).unwrap(), polygon);
}

#[cfg(feature = "geojson")]
#[test]
fn geojson_bounding_boxes_are_held_to_rfc_7946() {
    let mut converted = <geojson::Geometry>::try_from(&parse(r#"{"type":"Point","coordinates":[0,0]}"#).unwrap()).unwrap();
    converted.bbox = Some(vec![0.0, 0.0, 1.0]);
    assert_eq!(Geometry::try_from(&converted), Err(GeometryError::InvalidBoundingBox));
    converted.bbox = Some(vec![0.0, 0.0, 200.0, 1.0]);
    assert_eq!(
        Geometry::try_from(&converted),
        Err(GeometryError::LongitudeOutOfRange { value: Number::from(200) })
    );
}
