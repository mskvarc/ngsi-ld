//! The MQTT notification binding (NGSI-LD clause 7): the message a broker publishes for a notification to a
//! subscription whose endpoint is an `mqtt` or `mqtts` URI.

#[cfg(feature = "http")]
use crate::http::{DeliveryError, delivery::context_of_link};
use crate::{
    model::{
        DocumentError,
        KeyValuePair,
        MemberError,
        NgsiLdMediaType,
        RawJson,
        reader::{object, once, parse_json, required, string, unexpected, within},
    },
    subscription::ReadErrorKind,
};
#[cfg(feature = "http")]
use iri_rs::IriRefBuf;
use jstrict::{Value as Json, object::Entry};
use std::{convert::Infallible, error::Error};

/// The metadata key naming the media type of the body (Table 7.2-2).
const CONTENT_TYPE: &str = "Content-Type";

/// The metadata key holding the `Link` to the `@context` of an `application/json` body (Table 7.2-2).
const LINK: &str = "Link";

/// An MQTT notification message (NGSI-LD clause 7.2): a JSON object whose `body` is the notification of clause 5.3.1
/// and whose `metadata` holds, as key-value pairs, what the HTTP binding sends as headers: `Content-Type`, the `Link`
/// to the `@context` of an `application/json` body, and one entry per `receiverInfo` pair of the subscription.
///
/// The crate has no MQTT client; a receiver hands the payload of the message it received to
/// [`MqttNotification::parse`], then the body to the parse method of the notification it subscribed to.
///
/// The metadata values are kept as given, as [`KeyValuePair`] keeps them: their syntax is the one of the HTTP header
/// each stands for.
///
/// ```
/// use ngsi_ld::{
///     model::{Entity, Lossless, NgsiLdMediaType, Output},
///     subscription::{MqttNotification, Notification},
/// };
///
/// let message = MqttNotification::parse(
///     r#"{
///       "metadata": {
///         "Content-Type": "application/json",
///         "Link": "<https://example.org/context.jsonld>; rel=\"http://www.w3.org/ns/json-ld#context\"; type=\"application/ld+json\"",
///         "X-Auth-Token": "secret"
///       },
///       "body": {
///         "id": "urn:ngsi-ld:Notification:1",
///         "type": "Notification",
///         "subscriptionId": "urn:ngsi-ld:Subscription:1",
///         "notifiedAt": "2024-05-01T10:00:00Z",
///         "data": []
///       }
///     }"#,
/// )?;
/// assert_eq!(message.content_type(), NgsiLdMediaType::Json);
/// assert_eq!(message.receiver_info("X-Auth-Token"), Some("secret"));
/// let notification =
///     Notification::<Vec<Entity<Output>>>::parse(message.body(), Lossless::Normalized)?;
/// assert!(notification.data().is_empty());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MqttNotification {
    /// The metadata, in the order written.
    metadata: Vec<KeyValuePair>,
    /// The media type of the body, read from the `Content-Type` metadata.
    content_type: NgsiLdMediaType,
    /// The notification, as JSON text.
    body: String,
}

impl MqttNotification {
    /// Reads an MQTT notification message.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] when the text is not a JSON object holding exactly `metadata`, an object of text
    /// values, and `body`, an object, or when the `Content-Type` is neither `application/json` nor
    /// `application/ld+json`, the two values of Table 7.2-2.
    pub fn parse(text: &str) -> Result<MqttNotification, DocumentError<ReadErrorKind<Infallible>>> {
        let mut metadata = None;
        let mut body = None;
        for Entry { key, value } in object(parse_json(text)?)? {
            let key = key.as_str();
            match key {
                "metadata" => once(&mut metadata, key, within(key, pairs(value))?)?,
                "body" => once(&mut body, key, within(key, body_object(value))?)?,
                other => {
                    return Err(unexpected(other));
                }
            }
        }
        let metadata = required(metadata, "metadata")?;
        let body = required(body, "body")?;
        let content_type = content_type(&metadata).map_err(|error| error.within("metadata"))?;
        Ok(MqttNotification { metadata, content_type, body })
    }

    /// The media type of the body: the `Content-Type` metadata, `application/json` when it is absent (Table 7.2-2).
    #[must_use]
    pub const fn content_type(&self) -> NgsiLdMediaType {
        self.content_type
    }

    /// The `Link` metadata naming the `@context` of an `application/json` body, written as the HTTP header is.
    #[must_use]
    pub fn link(&self) -> Option<&str> {
        self.value(LINK)
    }

    /// The `@context` the `Link` metadata names, when there is one.
    ///
    /// # Errors
    ///
    /// Returns [`DeliveryError::Link`] when the `Link` cannot be read.
    #[cfg(feature = "http")]
    pub fn context(&self) -> Result<Option<IriRefBuf>, DeliveryError> {
        self.link().map_or(Ok(None), context_of_link)
    }

    /// The metadata entry the subscription's `receiverInfo` named `key` (clause 7.2).
    #[must_use]
    pub fn receiver_info(&self, key: &str) -> Option<&str> {
        self.value(key)
    }

    /// Every metadata entry, in the order written.
    #[must_use]
    pub fn metadata(&self) -> &[KeyValuePair] {
        &self.metadata
    }

    /// The notification, as JSON text: the parse method of the notification the subscription asked for reads it.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }

    /// The value of the metadata entry `key`.
    fn value(&self, key: &str) -> Option<&str> {
        self.metadata.iter().find(|pair| pair.key() == key).map(KeyValuePair::value)
    }
}

/// Reads `metadata`: an object whose every value is text.
fn pairs<P: Error>(json: Json) -> Result<Vec<KeyValuePair>, DocumentError<ReadErrorKind<P>>> {
    object(json)?
        .into_iter()
        .map(|Entry { key, value }| {
            let text = within(key.as_str(), string(value, "metadata"))?;
            Ok(KeyValuePair::new(key.as_str(), text))
        })
        .collect()
}

/// Reads `body`: the notification object, written back as JSON text for the notification's own reader.
fn body_object<P: Error>(json: Json) -> Result<String, DocumentError<ReadErrorKind<P>>> {
    match json {
        json @ Json::Object(_) => Ok(RawJson::from_json(json).to_string()),
        Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Array(_) => Err(DocumentError::top(MemberError::NotAnObject.into())),
    }
}

/// The media type the `Content-Type` metadata names.
fn content_type<P: Error>(metadata: &[KeyValuePair]) -> Result<NgsiLdMediaType, DocumentError<ReadErrorKind<P>>> {
    let Some(pair) = metadata.iter().find(|pair| pair.key() == CONTENT_TYPE) else {
        return Ok(NgsiLdMediaType::Json);
    };
    let invalid = || {
        DocumentError::top(
            MemberError::InvalidToken {
                member: CONTENT_TYPE,
                found: pair.value().to_owned(),
            }
            .into(),
        )
        .within(CONTENT_TYPE)
    };
    match pair.value().parse::<NgsiLdMediaType>() {
        Ok(media_type @ (NgsiLdMediaType::Json | NgsiLdMediaType::JsonLd)) => Ok(media_type),
        Ok(NgsiLdMediaType::GeoJson) | Err(_) => Err(invalid()),
    }
}

#[cfg(test)]
mod tests;
