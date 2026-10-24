// A `&mut Instances` hands out no mutable instance, since changing a dataset id through it could break the
// rules of clause 4.5.5.
use ngsi_ld::model::{Attribute, BuildError, Input, Instances, Property};

fn main() -> Result<(), BuildError> {
    let attribute: Attribute<Input> = Property::builder(1).build()?.into();
    let mut instances = Instances::one(attribute);
    for instance in &mut instances {
        let _ = instance;
    }
    Ok(())
}
