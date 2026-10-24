//! `csf` belongs to an entity subscription only (Table 5.2.12-1), so the builder of a Context Source
//! Registration Subscription has no setter for it.

use ngsi_ld::subscription::{Endpoint, NotificationParams, Subscription};

fn main() {
    let notification = NotificationParams::builder(Endpoint::builder("http://my.endpoint.org/notify").build().unwrap())
        .build()
        .unwrap();
    let _ = Subscription::registration_builder()
        .csf("owner".parse().unwrap())
        .notification(notification)
        .build();
}
