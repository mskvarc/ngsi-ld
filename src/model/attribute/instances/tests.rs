//! The clause 4.5.5 rules `Instances` keeps, whichever instance the default or a repeated dataset id collides with,
//! and at a size where checking every instance against every other would not finish.

use crate::model::{Attribute, AttributeKind, Change, DatasetId, Deletion, InstanceDeletion, Instances, InstancesError, Patch};
use pretty_assertions::assert_eq;

/// An instance: the deletion of the Property instance `dataset_id` names, or of its default instance.
type Instance = Change<Attribute<Patch>>;

fn named(number: usize) -> DatasetId {
    format!("urn:ngsi-ld:dataset:{number}").parse().unwrap()
}

fn instance(dataset_id: Option<DatasetId>) -> Instance {
    let deletion = InstanceDeletion::new(AttributeKind::Property);
    Change::Delete(Deletion::Instance(match dataset_id {
        Some(dataset_id) => deletion.with_dataset_id(dataset_id),
        None => deletion,
    }))
}

fn instances(count: usize) -> Instances<Instance> {
    Instances::try_from_vec((0..count).map(|number| instance(Some(named(number)))).collect()).unwrap()
}

#[test]
fn a_repeat_of_the_first_dataset_id_is_refused() {
    let mut instances = Instances::one(instance(Some(named(0))));
    instances.push(instance(Some(named(1)))).unwrap();
    assert_eq!(
        instances.push(instance(Some(named(0)))),
        Err(InstancesError::DuplicateDatasetId { dataset_id: named(0) })
    );
    assert_eq!(instances.len(), 2);
}

#[test]
fn a_second_default_instance_is_refused_whichever_spelling_either_uses() {
    for (first, second) in [(None, None), (None, Some(DatasetId::DefaultInstance)), (Some(DatasetId::DefaultInstance), None)] {
        let mut instances = Instances::one(instance(first));
        instances.push(instance(Some(named(1)))).unwrap();
        assert_eq!(instances.push(instance(second)), Err(InstancesError::DefaultTwice));
    }
}

#[test]
fn a_default_instance_added_after_named_ones_is_the_only_one_allowed() {
    let mut instances = instances(3);
    instances.push(instance(None)).unwrap();
    assert_eq!(instances.push(instance(Some(DatasetId::DefaultInstance))), Err(InstancesError::DefaultTwice));
    assert_eq!(instances.default_instance(), Some(&instance(None)));
}

#[test]
fn nothing_is_added_beside_a_bare_deletion() {
    let mut instances = Instances::one(Change::Delete(Deletion::Member));
    assert_eq!(instances.push(instance(Some(named(0)))), Err(InstancesError::BareDeletionAmongInstances));
    let mut named_first = Instances::one(instance(Some(named(0))));
    assert_eq!(
        named_first.push(Change::Delete(Deletion::Member)),
        Err(InstancesError::BareDeletionAmongInstances)
    );
}

#[test]
fn fifty_thousand_instances_keep_every_dataset_id_once() {
    let mut instances = instances(50_000);
    assert_eq!(instances.len(), 50_000);
    for repeated in [0, 25_000, 49_999] {
        assert_eq!(
            instances.push(instance(Some(named(repeated)))),
            Err(InstancesError::DuplicateDatasetId { dataset_id: named(repeated) })
        );
    }
    instances.push(instance(Some(named(50_000)))).unwrap();
    assert_eq!(instances.len(), 50_001);
}

#[test]
fn instances_are_equal_however_they_were_built() {
    let pushed = {
        let mut pushed = Instances::one(instance(Some(named(0))));
        pushed.push(instance(Some(named(1)))).unwrap();
        pushed
    };
    assert_eq!(pushed, instances(2));
    assert_eq!(Instances::one(instance(None)), Instances::try_from_vec(vec![instance(None)]).unwrap());
    assert_ne!(pushed, instances(3));
}
