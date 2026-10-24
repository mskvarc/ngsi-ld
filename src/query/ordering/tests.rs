use crate::{
    model::{
        LanguageTagError,
        geometry::{LineString, Position, Shape},
    },
    query::{
        Expected,
        ParseError,
        Problem,
        ordering::{self, OrderBy, OrderingKey, OrderingTerm, SortDirection, builders::EntityOrderingBuilder},
        projection::EntityMember,
        q::{self, AttributePath},
    },
};
use pretty_assertions::assert_eq;

/// Parses `text`, checks it prints back unchanged, and returns the ordering terms.
fn round_trip(text: &str) -> OrderBy {
    let order_by: OrderBy = text.parse().unwrap();
    assert_eq!(order_by.to_string(), text);
    order_by
}

/// A point at the given coordinates.
fn point(longitude: f64, latitude: f64) -> Shape {
    Shape::Point(Position::try_new(longitude, latitude).unwrap())
}

#[test]
fn or_01_one_attribute_ascending_by_default() {
    let order_by = round_trip("name");
    assert_eq!(ordering::by("name").build().unwrap().order_by(), &order_by);
    assert!(matches!(order_by.terms(), [term] if term.direction().is_none()));
}

#[test]
fn or_02_terms_apply_in_turn() {
    let order_by = round_trip("name,age");
    assert_eq!(ordering::by("name").then("age").build().unwrap().order_by(), &order_by);
}

#[test]
fn or_03_a_term_takes_a_direction() {
    let order_by = round_trip("name,age;desc");
    assert_eq!(ordering::by("name").then_descending("age").build().unwrap().order_by(), &order_by);
    assert_eq!(order_by.terms().get(1).and_then(OrderingTerm::direction), Some(SortDirection::Descending));
}

#[test]
fn or_04_a_member_path_into_a_compound_value() {
    let order_by = round_trip("address[city]");
    assert_eq!(ordering::by(q::attr("address").member("city")).build().unwrap().order_by(), &order_by);
}

#[test]
fn or_05_a_dotted_attribute_path() {
    let order_by = round_trip("name.observedAt");
    assert_eq!(ordering::by(q::attr("name").sub_attr("observedAt")).build().unwrap().order_by(), &order_by);
}

#[test]
fn or_06_and_07_a_collation() {
    for collation in ["und-u-ks-identic", "de-u-co-phonebk"] {
        let ordering = ordering::by("name").collation(collation).build().unwrap();
        assert_eq!(ordering.collation().map(ToString::to_string).as_deref(), Some(collation));
    }
}

#[test]
fn or_08_and_09_distance_from_a_point() {
    let nearest = ordering::nearest("location").origin(point(8.0, 40.0)).build().unwrap();
    assert_eq!(nearest.order_by(), &round_trip("location;dist-asc"));
    assert_eq!(nearest.origin(), Some(&point(8.0, 40.0)));
    let farthest = ordering::farthest("location").origin(point(8.0, 40.0)).build().unwrap();
    assert_eq!(farthest.order_by(), &round_trip("location;dist-desc"));
}

#[test]
fn or_10_distance_from_a_line_string() {
    let positions = [(8.0, 40.0), (9.0, 42.0), (9.0, 45.0), (8.0, 40.0)]
        .into_iter()
        .map(|(longitude, latitude)| Position::try_new(longitude, latitude).unwrap())
        .collect();
    let line = Shape::LineString(LineString::try_new(positions).unwrap());
    let ordering = ordering::nearest("location").origin(line.clone()).build().unwrap();
    assert_eq!(ordering.origin(), Some(&line));
}

#[test]
fn or_11_every_direction_and_entity_members() {
    let order_by = round_trip("id;asc,type;desc,scope,location;dist-asc,location;dist-desc");
    let keys: Vec<&OrderingKey> = order_by.terms().iter().map(OrderingTerm::key).collect();
    assert!(matches!(
        keys.as_slice(),
        [
            OrderingKey::Member(EntityMember::Id),
            OrderingKey::Member(EntityMember::Type),
            OrderingKey::Member(EntityMember::Scope),
            OrderingKey::Attribute(_),
            OrderingKey::Attribute(_)
        ]
    ));
    let built = ordering::ascending("id")
        .then_descending("type")
        .then("scope")
        .then_nearest("location")
        .then_farthest("location")
        .origin(point(8.0, 40.0))
        .build()
        .unwrap();
    assert_eq!(built.order_by(), &order_by);
}

#[test]
fn or_12_sorting_by_distance_needs_an_origin_and_an_attribute() {
    let error = ordering::nearest("location").build().unwrap_err();
    assert_eq!(error.problems(), [Problem::DistanceWithoutOrigin]);
    let error = ordering::nearest("id").origin(point(8.0, 40.0)).build().unwrap_err();
    assert_eq!(error.problems(), [Problem::DistanceOnEntityMember { member: EntityMember::Id }]);
    let parsed: OrderBy = "location;dist-asc".parse().unwrap();
    let error = EntityOrderingBuilder::from(parsed).build().unwrap_err();
    assert_eq!(error.problems(), [Problem::DistanceWithoutOrigin]);
}

#[test]
fn or_13_invalid_keys_and_collations_are_all_reported() {
    let error = ordering::by("9lives")
        .then(q::attr("sensor").linked("humidity"))
        .collation("not a tag")
        .build()
        .unwrap_err();
    assert!(matches!(
        error.problems(),
        [
            Problem::InvalidName { .. },
            Problem::LinkedOrderingKey {
                path: AttributePath::Linked(_)
            },
            Problem::InvalidCollation { .. }
        ]
    ));
}

#[test]
fn or_14_parse_errors_name_what_was_expected() {
    assert!(matches!(
        "name;up".parse::<OrderBy>(),
        Err(ParseError::Unexpected {
            expected: Expected::SortDirection,
            position: 5
        })
    ));
    assert!(matches!(
        "name,".parse::<OrderBy>(),
        Err(ParseError::Unexpected {
            expected: Expected::OrderingKey,
            position: 5
        })
    ));
    assert!(matches!(
        "name|age".parse::<OrderBy>(),
        Err(ParseError::Unexpected {
            expected: Expected::OrderingSeparator,
            position: 4
        })
    ));
    assert!(matches!("sensor{humidity}".parse::<OrderBy>(), Err(ParseError::Unexpected { position: 6, .. })));
}

#[test]
fn or_13_a_collation_that_is_no_language_tag_is_refused_as_a_collation() {
    let error = ordering::by("speed").collation("de u").build().unwrap_err();
    assert_eq!(
        error.problems(),
        [Problem::InvalidCollation {
            source: LanguageTagError::InvalidCharacter { found: ' ', position: 2 }
        }]
    );
}
