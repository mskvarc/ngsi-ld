//! `temporalQ` belongs to a Context Source Registration Subscription only (Table 5.2.12-1), so the builder of
//! an entity subscription has no setter for it.

use ngsi_ld::{
    query::temporal_query::TemporalQuery,
    subscription::{Endpoint, NotificationParams, Subscription},
};

fn main() {
    let time = "2024-05-01T10:00:00Z".parse().unwrap();
    let notification = NotificationParams::builder(Endpoint::builder("http://my.endpoint.org/notify").build().unwrap())
        .build()
        .unwrap();
    let _ = Subscription::builder()
        .temporal_q(TemporalQuery::after(time))
        .notification(notification)
        .build();
}
