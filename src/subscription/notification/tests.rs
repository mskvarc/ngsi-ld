//! Unit tests of notifications (clause 5.3.1) in each representation.

use crate::{
    model::{
        Attribute,
        AttributeName,
        DocumentError,
        DocumentErrorKind,
        Entity,
        FeatureCollection,
        GeoValue,
        Input,
        Lossless,
        MemberError,
        NullMember,
        Output,
        PathSegment,
        Projected,
        SimplifiedEntity,
        Value,
    },
    subscription::{DocumentType, Notification, ReadErrorKind},
};
use pretty_assertions::assert_eq;
use std::convert::Infallible;

/// The members of a notification before its `data`.
const HEAD: &str =
    r#""id":"urn:ngsi-ld:Notification:1","type":"Notification","subscriptionId":"urn:ngsi-ld:Subscription:mySubscription","notifiedAt":"2024-05-01T10:00:01Z""#;

/// A notification whose `data` is `data`.
fn notification(data: &str) -> String {
    format!(r#"{{{HEAD},"data":{data}}}"#)
}

/// Two vehicles in normalized form, the second parked.
const NORMALIZED: &str = concat!(
    r#"[{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":{"type":"Property","value":55}},"#,
    r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","isParked":{"type":"Relationship","object":"urn:ngsi-ld:OffStreetParking:Downtown1"}}]"#
);

/// The same vehicles in concise form.
const CONCISE: &str = concat!(
    r#"[{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":55},"#,
    r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","isParked":{"object":"urn:ngsi-ld:OffStreetParking:Downtown1"}}]"#
);

/// The deleted attributes of every kind, with their previous values, in normalized form.
const DELETED_NORMALIZED: &str = concat!(
    r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","#,
    r#""speed":{"type":"Property","value":"urn:ngsi-ld:null","previousValue":80,"deletedAt":"2024-05-01T10:00:00Z"},"#,
    r#""location":{"type":"GeoProperty","value":"urn:ngsi-ld:null","previousValue":{"type":"Point","coordinates":[8,40]},"deletedAt":"2024-05-01T10:00:00Z"},"#,
    r#""street":{"type":"LanguageProperty","languageMap":{"@none":"urn:ngsi-ld:null"},"previousLanguageMap":{"fr":"Grand Place"},"deletedAt":"2024-05-01T10:00:00Z"},"#,
    r#""gender":{"type":"VocabProperty","vocab":"urn:ngsi-ld:null","previousVocab":"Female","deletedAt":"2024-05-01T10:00:00Z"},"#,
    r#""tyres":{"type":"ListProperty","valueList":["urn:ngsi-ld:null"],"previousValueList":[1,2],"deletedAt":"2024-05-01T10:00:00Z"},"#,
    r#""tickets":{"type":"JsonProperty","json":"urn:ngsi-ld:null","previousJson":{"a":1},"deletedAt":"2024-05-01T10:00:00Z"},"#,
    r#""isParked":{"type":"Relationship","object":"urn:ngsi-ld:null","previousObject":"urn:ngsi-ld:OffStreetParking:Downtown1","deletedAt":"2024-05-01T10:00:00Z"},"#,
    r#""passengers":{"type":"ListRelationship","objectList":[{"object":"urn:ngsi-ld:null"}],"previousObjectList":[{"object":"urn:ngsi-ld:Person:Alice"}],"deletedAt":"2024-05-01T10:00:00Z"}}"#
);

/// The entity of `DELETED_NORMALIZED` written concise.
const DELETED_CONCISE: &str = concat!(
    r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","#,
    r#""speed":{"value":"urn:ngsi-ld:null","previousValue":80,"deletedAt":"2024-05-01T10:00:00Z"},"#,
    r#""location":{"type":"GeoProperty","value":"urn:ngsi-ld:null","previousValue":{"type":"Point","coordinates":[8,40]},"deletedAt":"2024-05-01T10:00:00Z"},"#,
    r#""street":{"languageMap":{"@none":"urn:ngsi-ld:null"},"previousLanguageMap":{"fr":"Grand Place"},"deletedAt":"2024-05-01T10:00:00Z"},"#,
    r#""gender":{"vocab":"urn:ngsi-ld:null","previousVocab":"Female","deletedAt":"2024-05-01T10:00:00Z"},"#,
    r#""tyres":{"valueList":["urn:ngsi-ld:null"],"previousValueList":[1,2],"deletedAt":"2024-05-01T10:00:00Z"},"#,
    r#""tickets":{"json":"urn:ngsi-ld:null","previousJson":{"a":1},"deletedAt":"2024-05-01T10:00:00Z"},"#,
    r#""isParked":{"object":"urn:ngsi-ld:null","previousObject":"urn:ngsi-ld:OffStreetParking:Downtown1","deletedAt":"2024-05-01T10:00:00Z"},"#,
    r#""passengers":{"objectList":["urn:ngsi-ld:null"],"previousObjectList":["urn:ngsi-ld:Person:Alice"],"deletedAt":"2024-05-01T10:00:00Z"}}"#
);

/// Reads a notification of complete entities in `form` and checks that it is written back as the same text.
fn round_trip(text: &str, form: Lossless) -> Notification<Vec<Entity<Output>>> {
    let read = Notification::<Vec<Entity<Output>>>::parse(text, form).unwrap();
    assert_eq!(read.render(form).to_string(), text);
    read
}

/// The attribute `name` of `entity`, its only instance.
fn attribute<'a>(entity: &'a Entity<Output>, name: &str) -> &'a Attribute<Output> {
    entity.attribute(&name.parse::<AttributeName>().unwrap()).unwrap().first()
}

#[test]
fn a_normalized_notification_round_trips() {
    let read = round_trip(&notification(NORMALIZED), Lossless::Normalized);
    assert_eq!(read.id().as_str(), "urn:ngsi-ld:Notification:1");
    assert_eq!(read.subscription_id().as_str(), "urn:ngsi-ld:Subscription:mySubscription");
    assert_eq!(read.notified_at().to_string(), "2024-05-01T10:00:01Z");
    assert_eq!(read.data().len(), 2);
    assert!(read.context().is_none());
}

#[test]
fn a_concise_notification_holds_the_same_entities() {
    let concise = round_trip(&notification(CONCISE), Lossless::Concise);
    let normalized = round_trip(&notification(NORMALIZED), Lossless::Normalized);
    assert_eq!(concise, normalized);
}

#[test]
fn a_simplified_notification_round_trips() {
    let text = notification(r#"[{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":55,"isParked":"urn:ngsi-ld:OffStreetParking:Downtown1"}]"#);
    let read = Notification::<Vec<SimplifiedEntity>>::parse(&text).unwrap();
    assert_eq!(read.render().to_string(), text);
    assert_eq!(read.data().len(), 1);
}

#[test]
fn a_geo_json_notification_carries_a_feature_collection() {
    let normalized = notification(concat!(
        r#"{"type":"FeatureCollection","features":[{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Feature","geometry":{"type":"Point","coordinates":[8,40]},"#,
        r#""properties":{"type":"Vehicle","location":{"type":"GeoProperty","value":{"type":"Point","coordinates":[8,40]}},"speed":{"type":"Property","value":55}}},"#,
        r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Feature","geometry":null,"properties":{"type":"Vehicle","speed":{"type":"Property","value":40}}}],"#,
        r#""@context":"https://example.org/context.jsonld"}"#
    ));
    let read = Notification::<FeatureCollection<Entity<Output>>>::parse(&normalized, Lossless::Normalized).unwrap();
    assert_eq!(read.render(Lossless::Normalized).to_string(), normalized);
    assert_eq!(read.data().features.len(), 2);
    assert!(read.data().context.is_some());
    let simplified = notification(concat!(
        r#"{"type":"FeatureCollection","features":[{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Feature","geometry":{"type":"Point","coordinates":[8,40]},"#,
        r#""properties":{"type":"Vehicle","location":{"type":"Point","coordinates":[8,40]},"speed":55}}]}"#
    ));
    let read = Notification::<FeatureCollection<SimplifiedEntity>>::parse(&simplified).unwrap();
    assert_eq!(read.render().to_string(), simplified);
    assert!(read.data().context.is_none());
}

#[test]
fn projected_entities_need_the_projected_reader() {
    let text = notification(r#"[{"speed":{"type":"Property","value":55}}]"#);
    let read = Notification::<Vec<Entity<Output, Projected>>>::parse(&text, Lossless::Normalized).unwrap();
    assert_eq!(read.render(Lossless::Normalized).to_string(), text);
    assert_eq!(read.data()[0].id, None);
    let error = Notification::<Vec<Entity<Output>>>::parse(&text, Lossless::Normalized).unwrap_err();
    assert_eq!(error.path.to_string(), "data.0");
    let ReadErrorKind::Entity { source } = error.kind else {
        panic!("expected an entity error, found {error}");
    };
    assert!(matches!(source.kind, DocumentErrorKind::Member(MemberError::MissingMember { member: "id" })));
}

#[test]
fn entities_a_pick_or_omit_reduced_are_read_in_every_form_whichever_of_id_and_type_remain() {
    let without_type = notification(r#"[{"id":"urn:ngsi-ld:Vehicle:A4567","speed":{"type":"Property","value":55}}]"#);
    let read = Notification::<Vec<Entity<Output, Projected>>>::parse(&without_type, Lossless::Normalized).unwrap();
    assert_eq!(read.render(Lossless::Normalized).to_string(), without_type);
    assert!(read.data()[0].id.is_some());
    assert_eq!(read.data()[0].entity_type, None);

    let without_id = notification(r#"[{"type":"Vehicle","speed":55}]"#);
    let read = Notification::<Vec<Entity<Output, Projected>>>::parse(&without_id, Lossless::Concise).unwrap();
    assert_eq!(read.render(Lossless::Concise).to_string(), without_id);
    assert_eq!(read.data()[0].id, None);
    assert!(read.data()[0].entity_type.is_some());

    let simplified = notification(r#"[{"speed":55},{"id":"urn:ngsi-ld:Vehicle:B9211","speed":40}]"#);
    let read = Notification::<Vec<SimplifiedEntity<Projected>>>::parse(&simplified).unwrap();
    assert_eq!(read.render().to_string(), simplified);
    assert_eq!(read.data().len(), 2);
}

#[test]
fn the_context_of_a_json_ld_notification_is_kept_last() {
    let text = format!(
        r#"{{{HEAD},"data":{NORMALIZED},"@context":["https://example.org/context.jsonld","https://uri.etsi.org/ngsi-ld/v1/ngsi-ld-core-context-v1.9.jsonld"]}}"#
    );
    let read = round_trip(&text, Lossless::Normalized);
    assert_eq!(read.context().map(|context| context.references().count()), Some(2));
}

#[test]
fn a_bare_null_in_normalized_form_is_a_deleted_property() {
    let text = notification(r#"[{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":"urn:ngsi-ld:null","location":"urn:ngsi-ld:null"}]"#);
    let read = round_trip(&text, Lossless::Normalized);
    let Attribute::Property(speed) = attribute(&read.data()[0], "speed") else {
        panic!("expected a Property");
    };
    assert_eq!(speed.value(), &Value::Text("urn:ngsi-ld:null".to_owned()));
    assert!(matches!(attribute(&read.data()[0], "location"), Attribute::Property(_)));
}

#[test]
fn deleted_attributes_of_every_kind_round_trip_in_normalized_form() {
    let read = round_trip(&notification(&format!("[{DELETED_NORMALIZED}]")), Lossless::Normalized);
    let entity = &read.data()[0];
    let kinds: Vec<_> = ["speed", "location", "street", "gender", "tyres", "tickets", "isParked", "passengers"]
        .into_iter()
        .map(|name| attribute(entity, name).kind().as_str())
        .collect();
    assert_eq!(
        kinds,
        [
            "Property",
            "GeoProperty",
            "LanguageProperty",
            "VocabProperty",
            "ListProperty",
            "JsonProperty",
            "Relationship",
            "ListRelationship"
        ]
    );
    let Attribute::GeoProperty(location) = attribute(entity, "location") else {
        panic!("expected a GeoProperty");
    };
    assert_eq!(location.value(), &GeoValue::Null);
    assert!(
        location
            .output()
            .previous_value
            .as_ref()
            .and_then(GeoValue::as_geometry)
            .is_some_and(|geometry| geometry.shape().is_point())
    );
}

#[test]
fn deleted_attributes_round_trip_in_concise_form_and_equal_the_normalized_ones() {
    let normalized = round_trip(&notification(&format!("[{DELETED_NORMALIZED}]")), Lossless::Normalized);
    let concise = notification(&format!("[{DELETED_CONCISE}]"));
    assert_eq!(normalized.render(Lossless::Concise).to_string(), concise);
    assert_eq!(round_trip(&concise, Lossless::Concise), normalized);
}

#[test]
fn every_instance_of_a_deleted_multi_attribute_is_kept_with_its_dataset_and_its_times() {
    let stamps = r#""createdAt":"2024-05-01T09:00:00Z","modifiedAt":"2024-05-01T10:00:00Z","deletedAt":"2024-05-01T10:00:00Z""#;
    let text = notification(&format!(
        concat!(
            r#"[{{"id":"urn:ngsi-ld:Building:1","type":"Building","label":["#,
            r#"{{"type":"Property","value":"urn:ngsi-ld:null",{stamps}}},"#,
            r#"{{"type":"Property","value":"urn:ngsi-ld:null","datasetId":"urn:ngsi-ld:Dataset:french",{stamps}}},"#,
            r#"{{"type":"Property","value":"urn:ngsi-ld:null","datasetId":"urn:ngsi-ld:Dataset:german",{stamps}}}],"#,
            r#""hosts":[{{"type":"Relationship","object":"urn:ngsi-ld:null",{stamps}}},"#,
            r#"{{"type":"Relationship","object":"urn:ngsi-ld:null","datasetId":"urn:ngsi-ld:Dataset:french",{stamps}}}]}}]"#
        ),
        stamps = stamps
    ));
    let read = round_trip(&text, Lossless::Normalized);
    let entity = &read.data()[0];
    let label = entity.attribute(&"label".parse::<AttributeName>().unwrap()).unwrap();
    assert_eq!(label.iter().count(), 3);
    assert_eq!(label.iter().filter(|instance| instance.meta().dataset_id.is_some()).count(), 2);
    let hosts = entity.attribute(&"hosts".parse::<AttributeName>().unwrap()).unwrap();
    assert_eq!(hosts.iter().count(), 2);
}

#[test]
fn a_deleted_attribute_with_nothing_to_report_is_written_as_the_bare_null_whichever_way_it_was_notified() {
    let object = notification(
        r#"[{"id":"urn:ngsi-ld:Building:1","type":"Building","hosts":{"type":"Relationship","object":"urn:ngsi-ld:null"},"label":{"type":"Property","value":"urn:ngsi-ld:null"}}]"#,
    );
    let bare = notification(r#"[{"id":"urn:ngsi-ld:Building:1","type":"Building","hosts":"urn:ngsi-ld:null","label":"urn:ngsi-ld:null"}]"#);
    let read = Notification::<Vec<Entity<Output>>>::parse(&object, Lossless::Normalized).unwrap();
    assert_eq!(attribute(&read.data()[0], "hosts").kind().as_str(), "Relationship");
    assert_eq!(read.render(Lossless::Normalized).to_string(), bare);
    assert_eq!(read.render(Lossless::Concise).to_string(), bare);
    let read = round_trip(&bare, Lossless::Normalized);
    assert_eq!(attribute(&read.data()[0], "hosts").kind().as_str(), "Property");
}

#[test]
fn a_concise_null_value_without_its_type_is_a_property() {
    let text = notification(
        r#"[{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","location":{"value":"urn:ngsi-ld:null","previousValue":{"type":"Point","coordinates":[8,40]}}}]"#,
    );
    let read = round_trip(&text, Lossless::Concise);
    assert!(matches!(attribute(&read.data()[0], "location"), Attribute::Property(_)));
}

#[test]
fn a_deleted_entity_carries_only_its_deletion_time() {
    let text = notification(r#"[{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","deletedAt":"2024-05-01T10:00:00Z"}]"#);
    let read = round_trip(&text, Lossless::Normalized);
    let entity = &read.data()[0];
    assert!(entity.attributes.is_empty());
    assert_eq!(
        entity.output.deleted_at.as_ref().map(ToString::to_string).as_deref(),
        Some("2024-05-01T10:00:00Z")
    );
}

/// The error of reading `text` as a normalized notification.
fn error(text: &str) -> DocumentError<ReadErrorKind<Infallible>> {
    Notification::<Vec<Entity<Output>>>::parse(text, Lossless::Normalized).unwrap_err()
}

#[test]
fn each_required_member_is_needed_and_unknown_members_refused() {
    let complete = notification("[]");
    for (member, written) in [
        ("id", r#""id":"urn:ngsi-ld:Notification:1","#),
        ("subscriptionId", r#","subscriptionId":"urn:ngsi-ld:Subscription:mySubscription""#),
        ("notifiedAt", r#","notifiedAt":"2024-05-01T10:00:01Z""#),
        ("data", r#","data":[]"#),
    ] {
        let found = error(&complete.replace(written, ""));
        assert!(
            matches!(found.kind, ReadErrorKind::Member(MemberError::MissingMember { member: missing }) if missing == member),
            "{member}: {found}"
        );
    }
    let wrong = error(&complete.replace(r#""type":"Notification""#, r#""type":"Subscription""#));
    assert!(matches!(
        wrong.kind,
        ReadErrorKind::WrongType {
            expected: DocumentType::Notification,
            ..
        }
    ));
    let object = error(&notification("{}"));
    assert!(matches!(object.kind, ReadErrorKind::Member(MemberError::NotAnArray)));
    assert_eq!(object.path.to_string(), "data");
    let array = Notification::<FeatureCollection<SimplifiedEntity>>::parse(&complete).unwrap_err();
    assert!(matches!(array.kind, ReadErrorKind::Entity { .. }));
    assert_eq!(array.path.to_string(), "data");
    let bad_id = error(&complete.replace("urn:ngsi-ld:Subscription:mySubscription", "not a uri"));
    assert!(matches!(bad_id.kind, ReadErrorKind::Member(MemberError::Identifier { .. })));
    assert_eq!(bad_id.path.to_string(), "subscriptionId");
    let reason = error(&complete.replace(r#""data":[]"#, r#""data":[],"triggerReason":"updated""#));
    assert!(matches!(reason.kind, ReadErrorKind::Member(MemberError::UnexpectedMember { .. })));
}

#[test]
fn an_entity_error_is_placed_under_data_and_the_entity_index() {
    let text = notification(
        r#"[{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle"},{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","speed":{"type":"Speed","value":1}}]"#,
    );
    let found = error(&text);
    assert_eq!(found.path.to_string(), "data.1");
    assert_eq!(
        found.path.segments().collect::<Vec<_>>(),
        [&PathSegment::Member("data".to_owned()), &PathSegment::Index(1)]
    );
    let ReadErrorKind::Entity { source } = found.kind else {
        panic!("expected an entity error, found {found}");
    };
    assert_eq!(source.path.to_string(), "speed");
    assert!(matches!(source.kind, DocumentErrorKind::UnknownAttributeType { .. }));
}

#[test]
fn a_request_refuses_the_null_of_a_deleted_attribute() {
    let null_member = |text: &str| {
        let error = Entity::<Input>::parse(text, Lossless::Normalized).unwrap_err();
        let DocumentErrorKind::NgsiLdNull { member } = error.kind else {
            panic!("expected the NGSI-LD Null, found {error}");
        };
        member
    };
    let geo = r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","location":{"type":"GeoProperty","value":"urn:ngsi-ld:null"}}"#;
    assert_eq!(null_member(geo), NullMember::Value);
    assert_eq!(
        null_member(r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":"urn:ngsi-ld:null"}"#),
        NullMember::Value
    );
    assert_eq!(
        null_member(r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","tickets":{"type":"JsonProperty","json":"urn:ngsi-ld:null"}}"#),
        NullMember::Json
    );
    let deleted = Entity::<Output>::parse(DELETED_NORMALIZED, Lossless::Normalized).unwrap();
    assert!(deleted.into_input().is_err());
}

#[test]
fn a_notification_posted_to_a_broker_by_a_context_source_is_read_and_written_back() {
    let core = "https://uri.etsi.org/ngsi-ld/v1/ngsi-ld-core-context-v1.9.jsonld";
    let bodies = [
        r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":{"type":"Property","value":40},"owner":{"type":"Relationship","object":"urn:ngsi-ld:Person:ada"},"colour":{"type":"Property","value":"red"}}"#,
        r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","name":{"type":"LanguageProperty","languageMap":{"en":"Tower","de":"Turm"}}}"#,
        r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":{"type":"Property","value":20,"observedAt":"2025-01-01T00:00:00Z"},"name":{"type":"LanguageProperty","languageMap":{"en":"Tower","de":"Turm"}}}"#,
        r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","owner":"urn:ngsi-ld:null","speed":{"type":"Property","value":40}}"#,
        r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","locatedIn":{"type":"Relationship","object":"urn:ngsi-ld:City:1"}}"#,
    ];
    for body in bodies {
        let text = format!(r#"{{{HEAD},"data":[{body}],"@context":"{core}"}}"#);
        let read = round_trip(&text, Lossless::Normalized);
        assert_eq!(read.data().len(), 1);
        assert!(read.context().is_some());
    }
}

#[test]
fn a_notified_entity_without_a_type_is_refused_by_the_complete_reader_and_read_by_the_projected_one() {
    let text = notification(
        r#"[{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":{"type":"Property","value":40}},{"id":"urn:ngsi-ld:Vehicle:B9211","speed":{"type":"Property","value":41}}]"#,
    );
    let refused = Notification::<Vec<Entity<Output>>>::parse(&text, Lossless::Normalized).unwrap_err();
    assert_eq!(refused.path.to_string(), "data.1");
    let read = Notification::<Vec<Entity<Output, Projected>>>::parse(&text, Lossless::Normalized).unwrap();
    assert_eq!(read.data().len(), 2);
    assert_eq!(read.render(Lossless::Normalized).to_string(), text);
}

#[test]
fn an_entity_of_a_notification_may_carry_its_own_context() {
    let text = notification(
        r#"[{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":{"type":"Property","value":40},"@context":"https://example.org/context.jsonld"}]"#,
    );
    let read = Notification::<Vec<Entity<Output>>>::parse(&text, Lossless::Normalized).unwrap();
    assert_eq!(read.render(Lossless::Normalized).to_string(), text);
}
