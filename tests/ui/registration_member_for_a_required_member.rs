//! A fragment cannot delete `endpoint` or `information`, which a registration needs; neither is a
//! `RegistrationMember`.

use ngsi_ld::registration::{RegistrationFragment, RegistrationMember};

fn main() {
    let _ = RegistrationFragment::builder().delete(RegistrationMember::Endpoint).build();
    let _ = RegistrationFragment::builder().delete(RegistrationMember::Information).build();
}
