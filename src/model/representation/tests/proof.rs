//! The digital signature of an attribute, `ngsildproof` (clauses 4.5.2.2 and 4.5.3.2, Annex C.11).

use crate::model::{
    Attribute,
    AttributeKind,
    AttributeName,
    DocumentErrorKind,
    Entity,
    EntityDirection,
    Input,
    Instances,
    Lossless,
    MemberError,
    NameError,
    Output,
    ProblemKind,
    Proof,
    Property,
    Representation,
    ReservedMember,
    Value,
};
use indexmap::IndexMap;
use iri_rs::iri;
use pretty_assertions::assert_eq;

/// A store whose `address` is signed, shaped like the example of Annex C.11 (this project's own values, not a copy
/// of the annex).
const STORE: &str = r#"{"id":"urn:ngsi-ld:Store:002","type":"Store","address":{"type":"Property","value":{"streetAddress":"Harbour Road 7","addressLocality":"Example Town","postalCode":"1000"},"ngsildproof":{"type":"Property","entityIdSealed":"urn:ngsi-ld:Store:002","entityTypeSealed":"Store","value":{"type":"DataIntegrityProof","created":"2025-01-27T21:02:24Z","verificationMethod":"https://example.org/issuers/1#key-1","cryptosuite":"eddsa-rdfc-2022","proofPurpose":"assertionMethod","proofValue":"z3Example"}}},"location":{"type":"GeoProperty","value":{"type":"Point","coordinates":[13.3986,52.5547]}},"@context":"https://example.org/store-context.jsonld"}"#;

/// The proof object of the examples.
fn signature() -> Value {
    Value::Object(IndexMap::from([
        ("type".to_owned(), Value::from("DataIntegrityProof")),
        ("cryptosuite".to_owned(), Value::from("eddsa-rdfc-2022")),
    ]))
}

/// The `ngsildproof` of the `address` attribute of an entity.
fn address_proof<D: EntityDirection>(entity: &Entity<D>) -> &Proof<D> {
    let name: AttributeName = "address".parse().unwrap();
    let Some(Attribute::Property(address)) = entity.attribute(&name).map(Instances::first) else {
        panic!("no address Property");
    };
    address.meta().proof.as_deref().unwrap()
}

#[test]
fn the_annex_c11_store_round_trips_in_normalized_form_in_both_directions() {
    let request = Entity::<Input>::parse(STORE, Lossless::Normalized).unwrap();
    assert_eq!(request.render(Representation::Normalized).to_string(), STORE);
    let response = Entity::<Output>::parse(STORE, Lossless::Normalized).unwrap();
    assert_eq!(response.render(Representation::Normalized).to_string(), STORE);
    let proof = address_proof(&response);
    assert_eq!(proof.entity_id_sealed().as_str(), "urn:ngsi-ld:Store:002");
    assert_eq!(proof.entity_type_sealed().as_one().map(ToString::to_string).as_deref(), Some("Store"));
    assert!(proof.property().value().is_object());
}

#[test]
fn a_proof_in_concise_form_keeps_its_sealed_members_and_reads_back_equal() {
    let request = Entity::<Input>::parse(STORE, Lossless::Normalized).unwrap();
    let concise = request.render(Representation::Concise).to_string();
    assert!(
        concise.contains(r#""ngsildproof":{"entityIdSealed":"urn:ngsi-ld:Store:002","entityTypeSealed":"Store","value":{"type":"DataIntegrityProof""#),
        "{concise}"
    );
    assert_eq!(Entity::<Input>::parse(&concise, Lossless::Concise).unwrap(), request);
}

#[test]
fn a_built_proof_is_written_after_the_shared_members() {
    let proof = Proof::builder(signature())
        .entity_id_sealed(iri!("urn:ngsi-ld:Store:002"))
        .entity_type_sealed("Store")
        .build()
        .unwrap();
    let address = Property::builder("Tiger Street 4")
        .observed_at("2025-01-27T21:02:24Z".parse().unwrap())
        .proof(proof)
        .build()
        .unwrap();
    assert_eq!(
        Attribute::from(address).render(Lossless::Normalized).to_string(),
        r#"{"type":"Property","value":"Tiger Street 4","observedAt":"2025-01-27T21:02:24Z","ngsildproof":{"type":"Property","entityIdSealed":"urn:ngsi-ld:Store:002","entityTypeSealed":"Store","value":{"type":"DataIntegrityProof","cryptosuite":"eddsa-rdfc-2022"}}}"#
    );
}

#[test]
fn the_proof_builder_reports_every_problem_together() {
    let error = Proof::builder("not an object")
        .entity_id_sealed("not a uri")
        .entity_type_sealed("1Store")
        .build()
        .unwrap_err();
    let kinds: Vec<&ProblemKind> = error.problems().iter().map(|problem| &problem.kind).collect();
    assert_eq!(kinds.len(), 3, "{error}");
    assert!(matches!(kinds[0], ProblemKind::Identifier { .. }));
    assert!(matches!(kinds[1], ProblemKind::Term { .. }));
    assert!(matches!(kinds[2], ProblemKind::ProofNotAnObject));
}

#[test]
fn the_reader_refuses_a_proof_out_of_shape() {
    let cases = [
        (
            r#"{"type":"Property","value":1,"ngsildproof":{"type":"Property","entityIdSealed":"urn:a:b","entityTypeSealed":"T","value":"text"}}"#,
            "not an object",
        ),
        (
            r#"{"type":"Property","value":1,"ngsildproof":{"type":"Relationship","entityIdSealed":"urn:a:b","entityTypeSealed":"T","object":"urn:a:c"}}"#,
            "not a Property",
        ),
        (
            r#"{"type":"Property","value":1,"ngsildproof":{"type":"Property","entityTypeSealed":"T","value":{}}}"#,
            "no entityIdSealed",
        ),
    ];
    for (text, case) in cases {
        let error = Attribute::<Output>::parse(text, Lossless::Normalized).unwrap_err();
        assert_eq!(error.path.to_string(), "ngsildproof", "{case}");
        match (case, error.kind) {
            ("not an object", DocumentErrorKind::ProofNotAnObject)
            | (
                "not a Property",
                DocumentErrorKind::ProofNotAProperty {
                    found: AttributeKind::Relationship,
                },
            )
            | ("no entityIdSealed", DocumentErrorKind::Member(MemberError::MissingMember { member: "entityIdSealed" })) => {}
            (case, other) => panic!("{case}: unexpected error {other}"),
        }
    }
}

#[test]
fn ngsildproof_cannot_name_an_attribute() {
    assert!(matches!(
        "ngsildproof".parse::<AttributeName>(),
        Err(NameError::Reserved {
            member: ReservedMember::NgsildProof
        })
    ));
}

#[test]
fn into_input_keeps_the_proof_and_drops_its_output_only_members() {
    let text = r#"{"id":"urn:ngsi-ld:Store:002","type":"Store","address":{"type":"Property","value":"Tiger Street 4","ngsildproof":{"type":"Property","entityIdSealed":"urn:ngsi-ld:Store:002","entityTypeSealed":"Store","value":{"type":"DataIntegrityProof"},"createdAt":"2025-01-27T21:02:24Z"}}}"#;
    let input = Entity::<Output>::parse(text, Lossless::Normalized).unwrap().into_input().unwrap();
    assert_eq!(
        input.render(Representation::Normalized).to_string(),
        r#"{"id":"urn:ngsi-ld:Store:002","type":"Store","address":{"type":"Property","value":"Tiger Street 4","ngsildproof":{"type":"Property","entityIdSealed":"urn:ngsi-ld:Store:002","entityTypeSealed":"Store","value":{"type":"DataIntegrityProof"}}}}"#
    );
}

#[test]
fn sealed_members_outside_an_ngsildproof_are_refused() {
    let error = Attribute::<Output>::parse(r#"{"type":"Property","value":1,"entityIdSealed":"urn:a:b"}"#, Lossless::Normalized).unwrap_err();
    assert!(matches!(error.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { ref member }) if member == "entityIdSealed"));
}

#[test]
fn a_fragment_carries_the_proof_of_the_attribute_it_sets() {
    let proof = Proof::builder(signature())
        .entity_id_sealed(iri!("urn:ngsi-ld:Store:002"))
        .entity_type_sealed("Store")
        .build()
        .unwrap();
    let patch = Attribute::from(Property::builder("Tiger Street 4").proof(proof).build().unwrap()).into_patch();
    assert_eq!(
        patch.render(Lossless::Concise).to_string(),
        r#"{"value":"Tiger Street 4","ngsildproof":{"entityIdSealed":"urn:ngsi-ld:Store:002","entityTypeSealed":"Store","value":{"type":"DataIntegrityProof","cryptosuite":"eddsa-rdfc-2022"}}}"#
    );
}
