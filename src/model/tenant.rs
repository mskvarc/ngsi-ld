//! Tenants (NGSI-LD clauses 4.14 and 5.5.10).

/// Why a string is not a tenant name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum TenantError {
    /// The name is empty.
    #[error("a tenant name must not be empty")]
    Empty,
    /// The name holds a character outside visible ASCII, which an HTTP header field cannot carry verbatim.
    #[error("a tenant name may only hold visible ASCII characters")]
    InvalidCharacter,
}

/// Checks a tenant name.
///
/// Clause 4.14 leaves the name a free string, but clause 6.3.14 carries it in the `NGSILD-Tenant` header, whose
/// value RFC 9110 section 5.5 restricts; visible ASCII without spaces is the part every binding can carry.
fn validate_tenant(name: &str) -> Result<(), TenantError> {
    if name.is_empty() {
        return Err(TenantError::Empty);
    }
    if !name.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(TenantError::InvalidCharacter);
    }
    Ok(())
}

/// The name of a tenant an operation targets (NGSI-LD clause 4.14). The default tenant has no name: an operation
/// on it carries no tenant at all.
///
/// ```
/// use ngsi_ld::model::Tenant;
///
/// let tenant: Tenant = "city-of-madrid".parse()?;
/// assert_eq!(tenant.to_string(), "city-of-madrid");
/// assert!("two words".parse::<Tenant>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[nutype::nutype(
    validate(with = validate_tenant, error = TenantError),
    derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Display, AsRef, FromStr, TryFrom, Serialize, Deserialize)
)]
pub struct Tenant(String);

#[cfg(test)]
mod tests {
    use crate::model::{Tenant, TenantError};

    #[test]
    fn a_tenant_of_visible_ascii_is_accepted_and_displayed_as_written() {
        for name in ["t1", "urn:ngsi-ld:tenant:city", "city_of-madrid.2"] {
            assert_eq!(name.parse::<Tenant>().map(|tenant| tenant.to_string()), Ok(name.to_owned()));
        }
    }

    #[test]
    fn an_empty_tenant_is_rejected() {
        assert_eq!("".parse::<Tenant>(), Err(TenantError::Empty));
    }

    #[test]
    fn a_tenant_with_a_space_or_non_ascii_character_is_rejected() {
        assert_eq!("two words".parse::<Tenant>(), Err(TenantError::InvalidCharacter));
        assert_eq!("münchen".parse::<Tenant>(), Err(TenantError::InvalidCharacter));
    }
}
