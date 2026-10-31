//! An `instanceId` names one instance of an evolution (clause 4.5.7): a request refuses it twice wherever the two
//! stand, and joining two answers keeps each identified instance once, at sizes where comparing every instance with
//! every other would not finish.

use crate::model::{
    Attribute,
    AttributeName,
    AttributesError,
    Input,
    InstanceId,
    Lossless,
    Output,
    Property,
    evolution::{TemporalAttributes, TemporalEntity},
};
use pretty_assertions::assert_eq;

fn speed() -> AttributeName {
    "speed".parse().unwrap()
}

fn instance_id(number: usize) -> InstanceId {
    format!("urn:ngsi-ld:i:{number}").parse().unwrap()
}

fn value(number: usize) -> Attribute<Input> {
    Attribute::Property(Property::builder(i64::try_from(number).unwrap()).build().unwrap())
}

#[test]
fn a_request_refuses_an_instance_id_named_by_any_earlier_instance() {
    let mut attributes = TemporalAttributes::new();
    for number in 0..20_000 {
        attributes.insert_identified(speed(), instance_id(number), value(number)).unwrap();
    }
    for repeated in [0, 10_000, 19_999] {
        assert_eq!(
            attributes.insert_identified(speed(), instance_id(repeated), value(repeated)),
            Err(AttributesError::InstanceIdRepeated)
        );
    }
    attributes.insert(speed(), value(1)).unwrap();
    attributes.insert(speed(), value(1)).unwrap();
    attributes.insert_identified(speed(), instance_id(20_000), value(20_000)).unwrap();
    assert_eq!(attributes.get(&speed()).unwrap().len(), 20_003);
}

/// An answer holding the `speed` instances `numbers`, each identified by its number.
fn page(numbers: impl Iterator<Item = usize>) -> TemporalEntity<Output> {
    let instances = numbers
        .map(|number| format!(r#"{{"type":"Property","value":{number},"instanceId":"urn:ngsi-ld:i:{number}"}}"#))
        .collect::<Vec<_>>()
        .join(",");
    TemporalEntity::<Output>::parse(
        &format!(r#"{{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","speed":[{instances}]}}"#),
        Lossless::Normalized,
    )
    .unwrap()
}

#[test]
fn joining_two_large_answers_keeps_each_identified_instance_once() {
    let mut first = page(0..20_000);
    first.absorb(page(19_990..40_000));
    let speeds = first.attribute(&speed()).unwrap();
    assert_eq!(speeds.len(), 40_000);
    assert!(
        speeds
            .iter()
            .enumerate()
            .all(|(number, instance)| instance.meta().output.instance_id.as_ref() == Some(&instance_id(number)))
    );
}
