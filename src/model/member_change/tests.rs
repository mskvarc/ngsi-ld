//! The changes of a fragment member.

use crate::model::MemberChange;
use jstrict::Value as Json;
use pretty_assertions::assert_eq;

#[test]
fn mc_01_a_set_member_lends_its_value_and_a_deleted_one_is_a_delete() {
    assert_eq!(MemberChange::Set(5).as_ref(), MemberChange::Set(&5));
    assert!(MemberChange::<u8>::Delete.is_delete());
    assert!(MemberChange::Set(5).is_set());
    assert_eq!(MemberChange::Set(5).map(|value| value * 2), MemberChange::Set(10));
    assert_eq!(MemberChange::<u8>::Delete.map(|value| value * 2), MemberChange::Delete);
}

#[test]
fn mc_01_a_deleted_member_is_written_as_the_ngsi_ld_null() {
    assert_eq!(MemberChange::<u8>::Delete.write(|_| None), Some(Json::from("urn:ngsi-ld:null")));
    assert_eq!(MemberChange::Set(5).write(|value| Some(Json::from(*value))), Some(Json::from(5)));
    assert_eq!(MemberChange::Set(5).write(|_| None), None);
}
