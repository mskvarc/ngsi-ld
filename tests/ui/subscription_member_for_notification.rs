//! A fragment cannot delete `notification`, which a subscription needs; it is no `SubscriptionMember`.

use ngsi_ld::subscription::{SubscriptionFragment, SubscriptionMember};

fn main() {
    let _ = SubscriptionFragment::builder().delete(SubscriptionMember::Notification).build();
    let _ = SubscriptionFragment::builder().delete(SubscriptionMember::Id).build();
}
