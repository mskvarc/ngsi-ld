//! Set Attribute Value (ETSI TS 104 175 clause 10.2.6 and ETSI TS 104 176 clause 7.6.3.2).

use crate::{
    http::{
        BatchOutcome,
        Endpoint,
        QueryParameter,
        RequestError,
        ResponseContext,
        ResponseError,
        operation::{
            ArgumentError,
            Operation,
            sealed::{BuildsRequest, Sealed},
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
        response::classify,
    },
    model::{AttributeName, DatasetId, DateTime, EntityId, RawJson, Value, null::holds_null},
    query::type_selection::TypeSelection,
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// The value Set Attribute Value sends: a value only Attribute Fragment, the attribute value itself (ETSI TS 104 175
/// clause 5.4.2).
///
/// ETSI TS 104 175 clause 10.2.6.4 refuses a value that holds JSON `null`, except the value of a `JsonProperty`, and
/// a value that holds the NGSI-LD Null anywhere. A [`Value`] holds no `null` by construction, so it is the value of
/// every kind of attribute but a `JsonProperty`, whose value is [`RawJson`]. The NGSI-LD Null is refused when the
/// request is built.
///
/// The enum is exhaustive: the exception of clause 10.2.6.4 makes exactly these two kinds of value.
///
/// ```
/// use ngsi_ld::{
///     http::operation::ValueOnly,
///     model::{RawJson, Value},
/// };
///
/// let speed = Value::from(30);
/// let ticket: RawJson = r#"{"id": "85a6cc52", "paid": null}"#.parse()?;
/// assert!(matches!(ValueOnly::from(&speed), ValueOnly::Value(_)));
/// assert!(matches!(ValueOnly::from(&ticket), ValueOnly::Json(_)));
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, derive_more::From)]
pub enum ValueOnly<'a> {
    /// The value of a Property (a JSON primitive, object or array), the target of a Relationship, the language map
    /// of a `LanguageProperty`, the vocabulary terms of a `VocabProperty`, or the list of a `ListProperty` or a
    /// `ListRelationship`.
    Value(&'a Value),
    /// The `json` of a `JsonProperty`, which may hold JSON `null`.
    Json(&'a RawJson),
}

impl ValueOnly<'_> {
    /// Whether the value is or holds the NGSI-LD Null.
    fn holds_null(self) -> bool {
        match self {
            Self::Value(value) => holds_null(value),
            Self::Json(json) => json.holds_ngsi_ld_null(),
        }
    }

    /// The value as sent.
    fn to_json(self) -> jstrict::Value {
        match self {
            Self::Value(value) => value.to_json(),
            Self::Json(json) => json.to_json(),
        }
    }
}

/// Sets the value of one attribute of an entity, keeping its kind and its other members.
///
/// Its request is `PUT /entities/{entityId}/attrs/{attrId}/value` (ETSI TS 104 176 clause 7.6.3.2), whose body is a
/// value only Attribute Fragment. The value cannot embed an `@context`, so it is always sent as `application/json`
/// with the `Link` header (ETSI TS 104 175 clause 5.4.2).
///
/// The value replaces the one of the instance `dataset_id` names, or else of the default instance; the broker answers
/// 404 when the entity has no such instance of the attribute. Where the attribute had an `observedAt`, the broker sets
/// it to `observed_at`, and removes it when there is none (ETSI TS 104 175 clause 10.2.6.4).
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, SetAttributeValue},
///     },
///     model::{AttributeName, DateTime, EntityId, Value},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
/// let speed: AttributeName = "speed".parse()?;
/// let slower = Value::from(30);
/// let observed: DateTime = "2024-05-01T10:00:00Z".parse()?;
/// let set = SetAttributeValue::builder()
///     .id(&id)
///     .name(&speed)
///     .value(&slower)
///     .observed_at(&observed)
///     .build();
/// let request = set.to_request(&endpoint)?;
/// assert_eq!(request.method(), "PUT");
/// assert_eq!(
///     request.uri(),
///     concat!(
///         "http://broker.test/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567/attrs/speed/value",
///         "?observedAt=2024-05-01T10%3A00%3A00Z"
///     )
/// );
/// assert_eq!(request.body().as_ref(), b"30");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, bon::Builder)]
pub struct SetAttributeValue<'a> {
    /// The entity.
    pub id: &'a EntityId,
    /// The attribute.
    pub name: &'a AttributeName,
    /// The new value.
    #[builder(into)]
    pub value: ValueOnly<'a>,
    /// Sets the value only if the entity's type matches (`type`, ETSI TS 104 175 clause 7.2.2).
    pub entity_type: Option<&'a TypeSelection>,
    /// The instance whose value is set, `@none` naming the default instance (`datasetId`); the default instance when
    /// `None`.
    pub dataset_id: Option<&'a DatasetId>,
    /// The time the value was observed (`observedAt`), used in preference to the server time for an attribute that
    /// had an `observedAt`.
    pub observed_at: Option<&'a DateTime>,
}

impl Sealed for SetAttributeValue<'_> {}

impl Operation for SetAttributeValue<'_> {
    type Outcome = BatchOutcome;

    /// Reads 204, or 207 when a distributed part of the operation failed (Table 7.6.3.2-2).
    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<BatchOutcome, ResponseError> {
        BatchOutcome::decode(&classify(response)?)
    }
}

impl BuildsRequest for SetAttributeValue<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        if self.value.holds_null() {
            return Err(ArgumentError::NullInValue.into());
        }
        let mut query = QueryString::default();
        query.push_type_selection(QueryParameter::Type, self.entity_type);
        query.push_some(QueryParameter::DatasetId, self.dataset_id);
        query.push_some(QueryParameter::ObservedAt, self.observed_at);
        RequestParts::resource(Method::PUT, ResourcePath::attribute_value(self.id, self.name), query)
            .with_value_only(self.value.to_json())
            .into_request(endpoint, scope)
    }
}
