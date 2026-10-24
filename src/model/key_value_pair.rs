//! A key and a value an endpoint conveys (NGSI-LD clause 5.2.22).
//!
//! The key stays free text: Table 5.2.22-1 types it as a string, and the pairs carry HTTP headers and MQTT user
//! properties, whose names the binding decides (MQTT user properties are arbitrary UTF-8).

use jstrict::{Object, Value as Json};

/// A key and a value an endpoint conveys, such as an HTTP header for the receiver.
///
/// NGSI-LD clause 5.2.22.
///
/// Both are text whose syntax the binding decides (Table 5.2.22-1): an HTTP header name and value for the HTTP
/// binding (clause 6.3.8), an MQTT setting for the MQTT one (clause 7.2), so the model keeps them as given.
///
/// ```
/// use ngsi_ld::model::KeyValuePair;
///
/// let prefer = KeyValuePair::new("Prefer", "body=json");
/// assert_eq!((prefer.key(), prefer.value()), ("Prefer", "body=json"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyValuePair {
    /// `key`.
    key: String,
    /// `value`.
    value: String,
}

impl KeyValuePair {
    /// The pair `key`, `value`.
    #[must_use]
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> KeyValuePair {
        KeyValuePair {
            key: key.into(),
            value: value.into(),
        }
    }

    /// Returns the key.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Returns the value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Writes the pair as `{"key": ..., "value": ...}`.
    pub(crate) fn to_json(&self) -> Json {
        let mut object = Object::new();
        object.push("key".into(), Json::from(self.key.as_str()));
        object.push("value".into(), Json::from(self.value.as_str()));
        Json::Object(object)
    }
}
