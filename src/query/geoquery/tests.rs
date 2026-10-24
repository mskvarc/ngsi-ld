use crate::{
    model::{
        JsonType,
        MemberError,
        Number,
        geometry::{GeometryError, Position, Shape},
    },
    query::{
        Expected,
        ParseError,
        Problem,
        geoquery::{self, Distance, DistanceError, GeoQuery, GeoQueryError, GeoRelation, NearDistance},
    },
};
use jstrict::{Print, Value as Json};
use pretty_assertions::assert_eq;

/// The Point `[8,40]` of clause 4.10 example 1.
fn point() -> Shape {
    Shape::Point(Position::try_new(8.0, 40.0).unwrap())
}

/// The Polygon of clause 4.10 example 2, read from its text so its lexical form is kept.
fn polygon() -> Shape {
    let coordinates: Json = "[[[100.0,0.0],[101.0,0.0],[101.0,1.0],[100.0,1.0],[100.0,0.0]]]".parse().unwrap();
    Shape::from_parts("Polygon", &coordinates).unwrap()
}

/// A distance from its lexical form.
fn distance(text: &str) -> Distance {
    text.parse().unwrap()
}

/// The coordinates a geoquery writes.
fn coordinates(query: &GeoQuery) -> String {
    query.geometry().coordinates_json().compact_print().to_string()
}

#[test]
fn example_1_near_with_a_maximum_distance_on_another_geoproperty() {
    let query = geoquery::near_max(2000, point()).geoproperty("observationSpace").build().unwrap();
    assert_eq!(query.relation(), &GeoRelation::Near(NearDistance::Max(distance("2000"))));
    assert_eq!(query.relation().to_string(), "near;maxDistance==2000");
    assert_eq!(query.geometry().type_name(), "Point");
    assert_eq!(coordinates(&query), "[8,40]");
    assert_eq!(query.geoproperty().map(AsRef::as_ref), Some("observationSpace"));
    assert_eq!("near;maxDistance==2000".parse::<GeoRelation>().unwrap(), *query.relation());
}

#[test]
fn example_2_within_a_polygon_keeps_the_lexical_coordinates() {
    let query = geoquery::within(polygon()).geoproperty("location").build().unwrap();
    assert_eq!(query.relation().to_string(), "within");
    assert_eq!(query.geometry().type_name(), "Polygon");
    assert_eq!(coordinates(&query), "[[[100.0,0.0],[101.0,0.0],[101.0,1.0],[100.0,1.0],[100.0,0.0]]]");
}

#[test]
fn example_3_names_no_geoproperty() {
    let query = geoquery::near_max(2000, point()).build().unwrap();
    assert_eq!(query.geoproperty(), None);
    assert_eq!(query, GeoQuery::new(GeoRelation::Near(NearDistance::Max(distance("2000"))), point()));
}

#[test]
fn a_minimum_distance_and_an_exponent_round_trip() {
    let min: GeoRelation = "near;minDistance==0.5".parse().unwrap();
    assert_eq!(min, GeoRelation::Near(NearDistance::Min(distance("0.5"))));
    assert_eq!(min.to_string(), "near;minDistance==0.5");
    let max: GeoRelation = "near;maxDistance==1e3".parse().unwrap();
    assert_eq!(max, GeoRelation::Near(NearDistance::Max(distance("1e3"))));
    assert_eq!(max.to_string(), "near;maxDistance==1e3");
    let decimal = geoquery::near_min(Number::try_from(0.5).unwrap(), point()).build().unwrap();
    assert_eq!(decimal.relation(), &min);
}

#[test]
fn every_other_relation_round_trips_and_equals_its_builder() {
    let built = [
        (geoquery::within(point()), GeoRelation::Within, "within"),
        (geoquery::contains(point()), GeoRelation::Contains, "contains"),
        (geoquery::intersects(point()), GeoRelation::Intersects, "intersects"),
        (geoquery::equals(point()), GeoRelation::Equals, "equals"),
        (geoquery::disjoint(point()), GeoRelation::Disjoint, "disjoint"),
        (geoquery::overlaps(point()), GeoRelation::Overlaps, "overlaps"),
    ];
    for (builder, relation, text) in built {
        assert_eq!(text.parse::<GeoRelation>().unwrap(), relation);
        assert_eq!(relation.to_string(), text);
        assert_eq!(builder.build().unwrap().relation(), &relation);
    }
}

#[test]
fn relations_outside_the_grammar_are_refused_where_they_go_wrong() {
    let cases = [
        ("near", Expected::DistanceModifier, 4),
        ("near;maxDistance=2000", Expected::DistanceModifier, 4),
        ("near;maxDistance==", Expected::PositiveNumber, 18),
        ("beside", Expected::GeoRelation, 0),
        ("Within", Expected::GeoRelation, 0),
        ("within;maxDistance==5", Expected::EndOfInput, 6),
        ("near;maxDistance==10;minDistance==1", Expected::EndOfInput, 20),
    ];
    for (text, expected, position) in cases {
        let error = text.parse::<GeoRelation>().unwrap_err();
        assert!(
            matches!(error, ParseError::Unexpected { expected: found, position: at } if found == expected && at == position),
            "{text}: {error:?}"
        );
    }
}

#[test]
fn a_distance_must_be_a_number_greater_than_zero() {
    let cases = [
        ("near;maxDistance==0", DistanceError::NotPositive),
        ("near;maxDistance==0.0e5", DistanceError::NotPositive),
        ("near;maxDistance==-5", DistanceError::NotPositive),
        ("near;maxDistance==05", DistanceError::NotANumber),
    ];
    for (text, reason) in cases {
        let error = text.parse::<GeoRelation>().unwrap_err();
        assert!(
            matches!(error, ParseError::InvalidDistance { position: 18, source } if source == reason),
            "{text}: {error:?}"
        );
    }
    assert_eq!(Distance::try_from(Number::from(0)), Err(DistanceError::NotPositive));
    assert_eq!(" 5".parse::<Distance>(), Err(DistanceError::NotANumber));
    assert_eq!(distance("1e-400").to_string(), "1e-400");
}

#[test]
fn the_builder_reports_a_distance_and_a_name_together() {
    let error = geoquery::near_max(0, point()).geoproperty("1loc").build().unwrap_err();
    assert!(
        matches!(
            error.problems(),
            [
                Problem::InvalidDistance {
                    source: DistanceError::NotPositive,
                    ..
                },
                Problem::InvalidName { .. },
            ]
        ),
        "{error:?}"
    );
}

#[test]
fn the_json_form_round_trips_with_and_without_a_geoproperty() {
    let with = r#"{"georel":"near;maxDistance==2000","geometry":"Point","coordinates":[8,40],"geoproperty":"observationSpace"}"#;
    let query = GeoQuery::parse(with).unwrap();
    assert_eq!(query, geoquery::near_max(2000, point()).geoproperty("observationSpace").build().unwrap());
    assert_eq!(query.render().to_string(), with);
    let without = r#"{"georel":"near;maxDistance==2000","geometry":"Point","coordinates":[8,40]}"#;
    let query = GeoQuery::parse(without).unwrap();
    assert_eq!(query, geoquery::near_max(2000, point()).build().unwrap());
    assert_eq!(query.render().to_string(), without);
}

#[test]
fn coordinates_written_as_a_string_are_read_and_written_as_the_array() {
    let query = GeoQuery::parse(r#"{"georel":"near;maxDistance==2000","geometry":"Point","coordinates":"[8,40]"}"#).unwrap();
    assert_eq!(query, geoquery::near_max(2000, point()).build().unwrap());
    assert_eq!(
        query.render().to_string(),
        r#"{"georel":"near;maxDistance==2000","geometry":"Point","coordinates":[8,40]}"#
    );
}

#[test]
fn every_geometry_type_round_trips_in_the_json_form() {
    let geometries = [
        ("MultiPoint", "[[8,40],[9,41]]"),
        ("LineString", "[[8,40],[9,41]]"),
        ("MultiLineString", "[[[8,40],[9,41]],[[10,42],[11,43]]]"),
        ("Polygon", "[[[100.0,0.0],[101.0,0.0],[101.0,1.0],[100.0,1.0],[100.0,0.0]]]"),
        ("MultiPolygon", "[[[[100.0,0.0],[101.0,0.0],[101.0,1.0],[100.0,1.0],[100.0,0.0]]]]"),
    ];
    for (geometry, coordinates) in geometries {
        let text = format!(r#"{{"georel":"intersects","geometry":"{geometry}","coordinates":{coordinates}}}"#);
        let query = GeoQuery::parse(&text).unwrap();
        assert_eq!(query.geometry().type_name(), geometry);
        assert_eq!(query.render().to_string(), text);
    }
}

#[test]
fn json_forms_outside_table_5_2_13_1_are_refused() {
    let point = r#""geometry":"Point","coordinates":[8,40]"#;
    let error = |text: &str| GeoQuery::parse(text).unwrap_err();
    assert!(matches!(
        error(&format!("{{{point}}}")).kind,
        GeoQueryError::Member(MemberError::MissingMember { member: "georel" })
    ));
    assert!(matches!(
        error(r#"{"georel":"within","geometry":"GeometryCollection","coordinates":[]}"#).kind,
        GeoQueryError::Member(MemberError::Geometry {
            source: GeometryError::GeometryCollection
        })
    ));
    assert!(matches!(
        error(r#"{"georel":"within","geometry":"Point","coordinates":[[8,40]]}"#).kind,
        GeoQueryError::Member(MemberError::Geometry {
            source: GeometryError::InvalidPosition
        })
    ));
    assert!(matches!(
        error(&format!(r#"{{"georel":"within",{point},"bbox":[8,40,9,41]}}"#)).kind,
        GeoQueryError::Member(MemberError::UnexpectedMember { member }) if member == "bbox"
    ));
    let relation = error(&format!(r#"{{"georel":"near",{point}}}"#));
    assert!(matches!(
        relation.kind,
        GeoQueryError::Relation {
            source: ParseError::Unexpected {
                expected: Expected::DistanceModifier,
                position: 4
            }
        }
    ));
    assert_eq!(relation.path.to_string(), "georel");
    let coordinates = error(r#"{"georel":"within","geometry":"Point","coordinates":"not json"}"#);
    assert!(matches!(coordinates.kind, GeoQueryError::CoordinatesNotJson));
    assert_eq!(coordinates.path.to_string(), "coordinates");
    let geoproperty = error(&format!(r#"{{"georel":"within",{point},"geoproperty":"1loc"}}"#));
    assert!(matches!(geoproperty.kind, GeoQueryError::Member(MemberError::Name { .. })));
    assert_eq!(geoproperty.path.to_string(), "geoproperty");
    assert!(matches!(
        error(&format!(r#"{{"georel":5,{point}}}"#)).kind,
        GeoQueryError::Member(MemberError::WrongShape {
            member: "georel",
            expected: JsonType::String
        })
    ));
    assert!(matches!(error("[]").kind, GeoQueryError::Member(MemberError::NotAnObject)));
    assert!(matches!(error("{").kind, GeoQueryError::Member(MemberError::NotJson)));
}

#[test]
fn the_annex_c4_coordinates_outside_wgs_84_are_read_as_written() {
    let text = r#"{"georel":"near;maxDistance==2000","geometry":"Point","coordinates":[-1,100]}"#;
    assert_eq!(GeoQuery::parse(text).unwrap().to_json().compact_print().to_string(), text);
}
