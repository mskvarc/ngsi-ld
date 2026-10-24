//! The `NGSILD-Tenant` header (NGSI-LD clause 6.3.14).

use crate::{
    http::headers::{InvalidFieldValue, text_field::TextField},
    model::Tenant,
};
use http::{HeaderName, HeaderValue};

/// The header name.
static NGSILD_TENANT: HeaderName = HeaderName::from_static("ngsild-tenant");

/// The `NGSILD-Tenant` header: the tenant an operation targets.
///
/// NGSI-LD clause 6.3.14.
///
/// The field value is rendered once, by [`NgsiLdTenant::try_new`], and kept: [`headers::Header::encode`] cannot fail,
/// so a tenant that could not be carried must be refused before encoding is reached.
///
/// ```
/// use headers::{Header, HeaderMapExt};
/// use http::HeaderMap;
/// use ngsi_ld::{http::headers::NgsiLdTenant, model::Tenant};
///
/// let tenant: Tenant = "city-of-madrid".parse()?;
/// let mut headers = HeaderMap::new();
/// headers.typed_insert(NgsiLdTenant::try_new(tenant)?);
/// assert_eq!(headers[NgsiLdTenant::name()], "city-of-madrid");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NgsiLdTenant(TextField<Tenant>);

impl NgsiLdTenant {
    /// An `NGSILD-Tenant` header naming `tenant`.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidFieldValue`] when the tenant cannot be carried in a header field.
    pub fn try_new(tenant: Tenant) -> Result<NgsiLdTenant, InvalidFieldValue> {
        TextField::new(tenant).map(NgsiLdTenant)
    }

    /// The tenant.
    #[must_use]
    pub const fn tenant(&self) -> &Tenant {
        self.0.value()
    }

    /// Consumes the header and returns the tenant.
    #[must_use]
    pub fn into_tenant(self) -> Tenant {
        self.0.into_value()
    }
}

impl headers::Header for NgsiLdTenant {
    fn name() -> &'static HeaderName {
        &NGSILD_TENANT
    }

    fn decode<'i, I>(values: &mut I) -> Result<NgsiLdTenant, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        TextField::decode(values).map(NgsiLdTenant)
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        self.0.encode(values);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        http::headers::{NgsiLdTenant, field_value::tests::decode_line},
        model::Tenant,
    };
    use headers::Header;
    use http::HeaderValue;
    use pretty_assertions::assert_eq;

    #[test]
    fn a_tenant_decodes_without_surrounding_whitespace() {
        assert_eq!(
            decode_line::<NgsiLdTenant>("t1").map(|header| header.tenant().to_string()),
            Some("t1".to_owned())
        );
        assert_eq!(
            decode_line::<NgsiLdTenant>("  t1  ").map(|header| header.tenant().to_string()),
            Some("t1".to_owned())
        );
    }

    #[test]
    fn an_empty_tenant_or_one_with_a_space_is_rejected() {
        assert_eq!(decode_line::<NgsiLdTenant>(""), None);
        assert_eq!(decode_line::<NgsiLdTenant>("   "), None);
        assert_eq!(decode_line::<NgsiLdTenant>("two words"), None);
    }

    #[test]
    fn a_tenant_sent_twice_is_rejected() {
        let lines = [HeaderValue::from_static("t1"), HeaderValue::from_static("t2")];
        assert!(NgsiLdTenant::decode(&mut lines.iter()).is_err());
    }

    #[test]
    fn a_tenant_encodes_as_written_and_decodes_back() {
        let header = NgsiLdTenant::try_new("urn:ngsi-ld:tenant:city".parse::<Tenant>().unwrap()).unwrap();
        let mut encoded = Vec::new();
        header.encode(&mut encoded);
        assert_eq!(encoded, vec![HeaderValue::from_static("urn:ngsi-ld:tenant:city")]);
        assert_eq!(NgsiLdTenant::decode(&mut encoded.iter()).ok(), Some(header.clone()));
        assert_eq!(header.into_tenant().to_string(), "urn:ngsi-ld:tenant:city");
    }
}
