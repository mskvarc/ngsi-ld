// Add Attributes requires an array of instances (clause 5.6.12.3), so its operation has no spelling to choose.
use ngsi_ld::{
    http::operation::AddTemporalAttributes,
    model::{
        EntityId,
        Input,
        evolution::{InstanceSpelling, TemporalAttributes},
    },
};

fn main() {
    let id: EntityId = "urn:ngsi-ld:Vehicle:B9211".parse().unwrap();
    let attributes = TemporalAttributes::<Input>::new();
    let _ = AddTemporalAttributes::builder()
        .id(&id)
        .attributes(&attributes)
        .spelling(InstanceSpelling::SingleWhenOne);
}
