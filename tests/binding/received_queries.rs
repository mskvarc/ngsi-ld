//! Query Entities and Retrieve Entity requests read back in typed form, as a context source reads the requests a
//! broker forwards to it (clauses 5.7.1.4 and 5.7.2.4): the query string of either operation (Tables 6.4.3.2-1 and
//! 6.5.3.1-1), and the `Query` body of a POST query (clause 5.2.23).

use crate::{
    fixtures::{endpoint, vehicle_id},
    query_vehicles,
    retrieve_entity,
};
use iri_rs::iri;
use jstrict::{Unordered, Value as Json};
use ngsi_ld::{
    http::{
        QueryParameter,
        RequestError,
        operation::{
            ArgumentError,
            EntityMapLifetime,
            EntityMapRequest,
            GeoJson,
            InlineJoin,
            Operation,
            ParameterError,
            QueryEntities,
            QueryEntitiesParameters,
            QueryEntitiesPostParameters,
            QueryMethod,
            RepresentationSpelling,
            RepresentationToken,
            ResultsCount,
            RetrieveEntity,
            RetrieveEntityParameters,
            Simplified,
            SystemAttributes,
        },
    },
    model::{
        AttributeName,
        DatasetId,
        EntityId,
        Lossless,
        OneOrMany,
        Representation,
        geometry::{LineString, Position, Shape},
    },
    query::{
        EntityMapReturn,
        EntitySelector,
        EntitySplitting,
        IdPattern,
        Join,
        Locality,
        SelectorType,
        geoquery::{self, GeoQuery},
        language_filter::LanguageFilter,
        ordering::{self, EntityOrdering},
        projection::Projection,
        q::{self, Query},
        scope_query::ScopeQuery,
        type_selection::TypeSelection,
    },
    snapshot::{EntityQuery, QueryDocument},
};
use pretty_assertions::assert_eq;
use std::{num::NonZeroU32, str};

/// The value sent in every parameter of a query, owned so that a request can borrow them.
struct Sent {
    entity_type: SelectorType,
    ids: [EntityId; 2],
    id_pattern: IdPattern,
    attrs: [AttributeName; 1],
    q: Query,
    geo_q: GeoQuery,
    scope_q: ScopeQuery,
    csf: Query,
    pick: Projection,
    omit: Projection,
    expand_values: [AttributeName; 1],
    json_keys: [AttributeName; 1],
    dataset_ids: [DatasetId; 2],
    lang: LanguageFilter,
    contained_by: [EntityId; 1],
    ordering: EntityOrdering,
    lifetime: EntityMapLifetime,
}

/// Sent for every parameter, each in a form its grammar makes non-trivial.
fn values() -> Sent {
    let point = Shape::Point(Position::try_new(8.0, 40.0).unwrap());
    let line = Shape::LineString(LineString::try_new(vec![Position::try_new(13.3986, 52.5547).unwrap(), Position::try_new(13.4, 52.6).unwrap()]).unwrap());
    Sent {
        entity_type: "(Home;Vehicle)|Motorhome".parse().unwrap(),
        ids: [
            EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567")),
            EntityId::from(iri!("urn:ngsi-ld:Vehicle:B,9211")),
        ],
        id_pattern: "^urn:ngsi-ld:Vehicle:A.*$|^urn:ngsi-ld:Vehicle:B.*$".parse().unwrap(),
        attrs: ["speed".parse().unwrap()],
        q: q::attr("speed").gt(50).and(q::attr("brandName").ne("Mercedes")).build().unwrap(),
        geo_q: geoquery::near_max(2000, point).geoproperty("observationSpace").build().unwrap(),
        scope_q: "(/Madrid/Districts;/CompanyA)|/Madrid/+/ParqueNorte".parse().unwrap(),
        csf: r#"provider=="urn:ngsi-ld:Provider:1""#.parse().unwrap(),
        pick: "id,type,speed,isParked{name}".parse().unwrap(),
        omit: "brandName".parse().unwrap(),
        expand_values: ["gender".parse().unwrap()],
        json_keys: ["parkingTickets".parse().unwrap()],
        dataset_ids: ["@none".parse().unwrap(), "urn:ngsi-ld:dataset:gps".parse().unwrap()],
        lang: "fr-CH,fr;q=0.9,*;q=0.5".parse().unwrap(),
        contained_by: ["urn:ngsi-ld:Building:1".parse().unwrap()],
        ordering: ordering::nearest("location").origin(line).collation("de-u-co-phonebk").build().unwrap(),
        lifetime: "PT1H".parse().unwrap(),
    }
}

/// A query with every parameter of `values`, in `format`.
fn full_query<'a, F>(values: &'a Sent, entity_map: &'a EntityMapRequest, format: F) -> QueryEntities<'a, F> {
    QueryEntities {
        ids: &values.ids,
        id_pattern: Some(&values.id_pattern),
        attrs: &values.attrs,
        geo_q: Some(&values.geo_q),
        scope_q: Some(&values.scope_q),
        csf: Some(&values.csf),
        pick: Some(&values.pick),
        omit: Some(&values.omit),
        expand_values: &values.expand_values,
        json_keys: &values.json_keys,
        dataset_ids: &values.dataset_ids,
        lang: Some(&values.lang),
        join: Some(Join::Inline),
        join_level: NonZeroU32::new(2),
        contained_by: &values.contained_by,
        ordering: Some(&values.ordering),
        split_entities: EntitySplitting::Split,
        entity_map: Some(entity_map),
        system_attributes: SystemAttributes::Include,
        limit: Some(20),
        offset: Some(40),
        count: ResultsCount::Include,
        locality: Locality::Local,
        ..query_vehicles(Some(&values.entity_type), Some(&values.q), format)
    }
}

#[test]
fn every_query_entities_parameter_reads_back_as_the_value_sent() {
    let values = values();
    let entity_map = EntityMapRequest::returned_for(values.lifetime);
    let query = QueryEntities {
        spelling: RepresentationSpelling::default().in_options().key_values(),
        ..full_query(&values, &entity_map, Simplified)
    };
    let request = query.to_request(&endpoint()).unwrap();
    let received = QueryEntitiesParameters::parse(request.uri().query().unwrap()).unwrap();
    assert_eq!(received.selection.entity_type.as_ref(), Some(&values.entity_type));
    assert_eq!(received.selection.ids, values.ids);
    assert_eq!(received.selection.id_pattern.as_ref(), Some(&values.id_pattern));
    assert_eq!(received.selection.attrs, values.attrs);
    assert_eq!(received.selection.q.as_ref(), Some(&values.q));
    assert_eq!(received.selection.geo_q.as_ref(), Some(&values.geo_q));
    assert_eq!(received.selection.scope_q.as_ref(), Some(&values.scope_q));
    assert_eq!(received.selection.csf.as_ref(), Some(&values.csf));
    assert_eq!(received.selection.pick.as_ref(), Some(&values.pick));
    assert_eq!(received.selection.omit.as_ref(), Some(&values.omit));
    assert_eq!(received.selection.expand_values, values.expand_values);
    assert_eq!(received.selection.json_keys, values.json_keys);
    assert_eq!(received.selection.dataset_ids, values.dataset_ids);
    assert_eq!(received.selection.lang.as_ref(), Some(&values.lang));
    assert_eq!(received.join, Some(Join::Inline));
    assert_eq!(received.join_level, NonZeroU32::new(2));
    assert_eq!(received.contained_by, values.contained_by);
    assert_eq!(received.assembly.ordering.as_ref(), Some(&values.ordering));
    assert_eq!(received.assembly.split_entities, EntitySplitting::Split);
    assert_eq!(received.assembly.entity_map, EntityMapReturn::Returned);
    assert_eq!(received.assembly.entity_map_lifetime, Some(values.lifetime));
    assert_eq!(received.representation, Representation::Simplified);
    assert_eq!(received.spelling, query.spelling);
    assert_eq!(received.system_attributes, SystemAttributes::Include);
    assert_eq!(received.geometry_property, None);
    assert_eq!((received.limit, received.offset, received.count), (Some(20), Some(40), ResultsCount::Include));
    assert_eq!(received.locality, Locality::Local);
}

#[test]
fn every_type_a_geo_json_property_and_an_explicit_format_read_back() {
    let every = SelectorType::All;
    let site: AttributeName = "site".parse().unwrap();
    let query = QueryEntities {
        spelling: RepresentationSpelling::default().explicit_normalized(),
        ..query_vehicles(
            Some(&every),
            None,
            GeoJson {
                form: Lossless::Normalized,
                geometry_property: Some(&site),
            },
        )
    };
    let request = query.to_request(&endpoint()).unwrap();
    assert_eq!(request.uri().query(), Some("type=*&format=normalized&geometryProperty=site"));
    let received = QueryEntitiesParameters::parse(request.uri().query().unwrap()).unwrap();
    assert_eq!(received.selection.entity_type, Some(SelectorType::All));
    assert_eq!(received.representation, Representation::Normalized);
    assert_eq!(received.spelling, RepresentationSpelling::default().explicit_normalized());
    assert_eq!(received.geometry_property, Some(site));
    assert_eq!(received.locality, Locality::Unspecified);
}

#[test]
fn every_retrieve_entity_parameter_reads_back_as_the_value_sent() {
    let values = values();
    let id = vehicle_id();
    let entity_type: TypeSelection = "Vehicle".parse().unwrap();
    let entity_map = EntityMapRequest::returned_for(values.lifetime);
    let retrieve = RetrieveEntity {
        entity_type: Some(&entity_type),
        attrs: &values.attrs,
        pick: Some(&values.pick),
        omit: Some(&values.omit),
        dataset_ids: &values.dataset_ids,
        lang: Some(&values.lang),
        join: Some(InlineJoin::Inline),
        join_level: NonZeroU32::new(3),
        contained_by: &values.contained_by,
        entity_map: Some(&entity_map),
        system_attributes: SystemAttributes::Include,
        spelling: RepresentationSpelling::default().in_options(),
        ..retrieve_entity(&id, Lossless::Concise)
    };
    let request = retrieve.to_request(&endpoint()).unwrap();
    let received = RetrieveEntityParameters::parse(request.uri().query().unwrap()).unwrap();
    assert_eq!(received.entity_type, Some(entity_type));
    assert_eq!(received.attrs, values.attrs);
    assert_eq!(received.pick.as_ref(), Some(&values.pick));
    assert_eq!(received.omit.as_ref(), Some(&values.omit));
    assert_eq!(received.dataset_ids, values.dataset_ids);
    assert_eq!(received.lang.as_ref(), Some(&values.lang));
    assert_eq!((received.join, received.join_level), (Some(Join::Inline), NonZeroU32::new(3)));
    assert_eq!(received.contained_by, values.contained_by);
    assert_eq!(
        (received.entity_map, received.entity_map_lifetime),
        (EntityMapReturn::Returned, Some(values.lifetime))
    );
    assert_eq!(received.representation, Representation::Concise);
    assert_eq!(received.spelling, RepresentationSpelling::default().in_options());
    assert_eq!(received.system_attributes, SystemAttributes::Include);
    assert_eq!(received.locality, Locality::Unspecified);
}

/// The error a query string of Query Entities is refused with.
fn refused(query: &str) -> ParameterError {
    QueryEntitiesParameters::parse(query).unwrap_err()
}

#[test]
fn an_invalid_query_string_is_refused_naming_the_parameter() {
    assert!(matches!(refused("type=Vehicle&foo=1"), ParameterError::Unknown { name } if name == "foo"));
    assert!(matches!(refused("q=a&q=b"), ParameterError::Repeated { parameter: QueryParameter::Q }));
    assert!(matches!(
        refused("q=speed%3E"),
        ParameterError::Syntax {
            parameter: QueryParameter::Q,
            ..
        }
    ));
    assert!(matches!(refused("idPattern=(a"), ParameterError::IdPattern { .. }));
    assert!(matches!(refused("join=sideways"), ParameterError::Join { .. }));
    assert!(matches!(
        refused("joinLevel=0"),
        ParameterError::Integer {
            parameter: QueryParameter::JoinLevel,
            ..
        }
    ));
    assert!(matches!(
        refused("limit=-1"),
        ParameterError::Integer {
            parameter: QueryParameter::Limit,
            ..
        }
    ));
    assert!(matches!(
        refused("local=yes"),
        ParameterError::NotABoolean {
            parameter: QueryParameter::Local
        }
    ));
    assert!(matches!(
        refused("id=not%20a%20uri"),
        ParameterError::Uri {
            parameter: QueryParameter::Id,
            ..
        }
    ));
    assert!(matches!(
        refused("attrs=9lives"),
        ParameterError::Name {
            parameter: QueryParameter::Attrs,
            ..
        }
    ));
    assert!(matches!(refused("georel=within"), ParameterError::GeoQuery { .. }));
    let lone = QueryEntitiesParameters::parse("type=Vehicle&geoproperty=location").unwrap();
    assert_eq!(
        (lone.selection.geo_q, lone.selection.lone_geoproperty.map(|name| name.to_string())),
        (None, Some("location".to_owned()))
    );
    assert!(matches!(refused("geometry=Point&coordinates=%5B8,40%5D"), ParameterError::GeoQuery { .. }));
    let level = QueryEntitiesParameters::parse("type=Vehicle&joinLevel=2").unwrap();
    assert_eq!((level.join, level.join_level), (None, NonZeroU32::new(2)));
    let uncounted = QueryEntitiesParameters::parse("type=Vehicle&count=false").unwrap();
    assert_eq!(uncounted.count, ResultsCount::Exclude);
    assert!(matches!(
        refused("orderFrom=%5B8,40%5D"),
        ParameterError::OrderingWithoutOrderBy {
            parameter: QueryParameter::OrderFrom
        }
    ));
    assert!(matches!(
        refused("orderBy=location;dist-asc&orderFrom=north"),
        ParameterError::CoordinatesNotJson {
            parameter: QueryParameter::OrderFrom
        }
    ));
    assert!(matches!(refused("orderBy=location;dist-asc"), ParameterError::Ordering { .. }));
    assert!(matches!(refused("entityMapLifetime=PT0S"), ParameterError::Lifetime { .. }));
    assert!(matches!(refused("options=concise,keyValues"), ParameterError::RepresentationTwice));
    let both = QueryEntitiesParameters::parse("format=concise&options=keyValues").unwrap();
    assert_eq!(both.representation, Representation::Concise);
    assert_eq!(
        both.spelling,
        RepresentationSpelling::default().format_over_options(RepresentationToken::KeyValues)
    );
    assert!(matches!(
        RetrieveEntityParameters::parse("type=*").unwrap_err(),
        ParameterError::Syntax {
            parameter: QueryParameter::Type,
            ..
        }
    ));
    assert!(matches!(
        RetrieveEntityParameters::parse("limit=1").unwrap_err(),
        ParameterError::Unknown { name } if name == "limit"
    ));
}

#[test]
fn a_post_query_body_reads_back_as_a_query_document_and_renders_the_same_json() {
    let values = values();
    let entity_map = EntityMapRequest::returned_for(values.lifetime);
    let selectors = [EntitySelector::new("Car".parse().unwrap()).with_id_pattern("^a$|^b$".parse().unwrap())];
    let both = QueryEntities {
        method: QueryMethod::Post,
        ..full_query(&values, &entity_map, Lossless::Normalized)
    };
    assert!(matches!(
        both.to_request(&endpoint()),
        Err(RequestError::Argument(ArgumentError::IdsWithIdPatternInSelector))
    ));
    let query = QueryEntities {
        id_pattern: None,
        selectors: &selectors,
        ..both
    };
    let request = query.to_request(&endpoint()).unwrap();
    let body = str::from_utf8(request.body()).unwrap();
    let document = QueryDocument::<EntityQuery>::parse(body).unwrap();
    let entities = document.entities();
    assert_eq!(entities.len(), 2);
    assert_eq!(entities[0].selector_type(), &values.entity_type);
    assert_eq!(entities[0].ids().map(OneOrMany::as_slice), Some(&values.ids[..]));
    assert_eq!(entities[0].id_pattern(), None);
    assert_eq!(entities[1], selectors[0]);
    assert_eq!(document.q(), Some(&values.q));
    assert_eq!(document.geo_q(), Some(&values.geo_q));
    assert_eq!(document.scope_q(), Some(&values.scope_q));
    assert_eq!(document.csf(), Some(&values.csf));
    assert_eq!(document.attrs(), values.attrs);
    assert_eq!(document.pick(), Some(&values.pick));
    assert_eq!(document.omit(), Some(&values.omit));
    assert_eq!(document.expand_values(), values.expand_values);
    assert_eq!(document.json_keys(), values.json_keys);
    assert_eq!(document.dataset_ids(), Some(values.dataset_ids.as_slice()));
    assert_eq!(document.lang(), Some(&values.lang));
    assert_eq!((document.join(), document.join_level()), (Some(Join::Inline), NonZeroU32::new(2)));
    assert_eq!(document.contained_by(), values.contained_by);
    assert_eq!(document.ordering(), Some(&values.ordering));
    assert_eq!(document.split_entities(), EntitySplitting::Split);
    assert_eq!(document.entity_map(), EntityMapReturn::Returned);
    assert_eq!(document.entity_map_lifetime().map(ToString::to_string).as_deref(), Some("PT1H"));
    let rendered: Json = document.render().to_string().parse().unwrap();
    let sent: Json = body.parse().unwrap();
    assert_eq!(Unordered(rendered), Unordered(sent));
    let sent = request.uri().query().unwrap();
    let received = QueryEntitiesPostParameters::parse(sent).unwrap();
    assert_eq!((received.limit, received.offset, received.locality), (Some(20), Some(40), Locality::Local));
    assert_eq!(received.representation, Representation::Normalized);
    assert_eq!(QueryEntitiesParameters::parse(sent).unwrap().selection.entity_type, None);
    for selection in ["type=Vehicle", "q=speed%3E50", "join=flat", "containedBy=urn:ngsi-ld:Vehicle:A"] {
        assert!(
            matches!(QueryEntitiesPostParameters::parse(selection), Err(ParameterError::Unknown { .. })),
            "{selection}"
        );
    }
}
