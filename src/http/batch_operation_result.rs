//! The result of an operation that succeeded only partly (NGSI-LD clauses 5.2.16 and 5.2.17).

use crate::model::{EntityId, ProblemDetails, RawJson, Uri};
use jstrict::{Object, Value as Json};
use serde::{Deserialize, Serialize};

/// Which entities an operation treated and which failed (NGSI-LD clause 5.2.16), the body of a 207 response.
///
/// Both members are mandatory in the specification; a broker that omits one is read as having listed nothing
/// there.
///
/// ```
/// use ngsi_ld::http::BatchOperationResult;
///
/// let result: BatchOperationResult = serde_json::from_str(
///     r#"{
///       "success": ["urn:ngsi-ld:Vehicle:A4567"],
///       "errors": [
///         {
///           "entityId": "urn:ngsi-ld:Vehicle:B9211",
///           "error": {"type": "https://uri.etsi.org/ngsi-ld/errors/AlreadyExists"}
///         }
///       ]
///     }"#,
/// )?;
/// assert_eq!(result.success.len(), 1);
/// assert_eq!(
///     result.errors[0].entity_id.as_str(),
///     "urn:ngsi-ld:Vehicle:B9211"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BatchOperationResult {
    /// The entities the operation treated successfully.
    #[serde(default)]
    pub success: Vec<EntityId>,
    /// One entry per entity in error.
    #[serde(default)]
    pub errors: Vec<BatchEntityError>,
}

/// An error tied to one entity of a batch or distributed operation (NGSI-LD clause 5.2.17).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchEntityError {
    /// The entity in error.
    pub entity_id: EntityId,
    /// What went wrong.
    pub error: ProblemDetails,
    /// The registration of the failed distributed operation, if it was one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration_id: Option<Uri>,
}

impl BatchOperationResult {
    /// Writes the result as the body of a 207 answer: `success`, then `errors`, each error with its `entityId`,
    /// its `error` and its `registrationId` when it has one (clauses 5.2.16 and 5.2.17).
    ///
    /// ```
    /// use iri_rs::iri;
    /// use ngsi_ld::{
    ///     http::{BatchEntityError, BatchOperationResult},
    ///     model::{ErrorType, ProblemDetails},
    /// };
    ///
    /// let result = BatchOperationResult {
    ///     success: vec![iri!("urn:ngsi-ld:Vehicle:A4567").into()],
    ///     errors: vec![BatchEntityError {
    ///         entity_id: iri!("urn:ngsi-ld:Vehicle:B9211").into(),
    ///         error: ProblemDetails::builder()
    ///             .error_type(ErrorType::AlreadyExists)
    ///             .title("Already exists")
    ///             .detail("urn:ngsi-ld:Vehicle:B9211")
    ///             .build(),
    ///         registration_id: None,
    ///     }],
    /// };
    /// assert_eq!(
    ///     result.render().to_string(),
    ///     concat!(
    ///         r#"{"success":["urn:ngsi-ld:Vehicle:A4567"],"#,
    ///         r#""errors":[{"entityId":"urn:ngsi-ld:Vehicle:B9211","#,
    ///         r#""error":{"type":"https://uri.etsi.org/ngsi-ld/errors"#,
    ///         r#"/AlreadyExists","title":"Already exists","#,
    ///         r#""detail":"urn:ngsi-ld:Vehicle:B9211"}}]}"#
    ///     )
    /// );
    /// ```
    #[must_use]
    pub fn render(&self) -> RawJson {
        let mut object = Object::new();
        let success = self.success.iter().map(|id| Json::from(id.as_str())).collect();
        object.push("success".into(), Json::Array(success));
        object.push("errors".into(), Json::Array(self.errors.iter().map(BatchEntityError::to_json).collect()));
        RawJson::from_json(Json::Object(object))
    }
}

impl BatchEntityError {
    /// Writes the error as an entry of `errors`.
    fn to_json(&self) -> Json {
        let mut object = Object::new();
        object.push("entityId".into(), Json::from(self.entity_id.as_str()));
        object.push("error".into(), self.error.to_json());
        if let Some(registration_id) = &self.registration_id {
            object.push("registrationId".into(), Json::from(registration_id.to_string()));
        }
        Json::Object(object)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        http::{BatchEntityError, BatchOperationResult},
        model::{ErrorType, ProblemDetails},
    };
    use iri_rs::iri;
    use pretty_assertions::assert_eq;

    #[test]
    fn a_result_with_successes_and_errors_round_trips() {
        let text = r#"{"success":["urn:ngsi-ld:Vehicle:A4567"],"errors":[{"entityId":"urn:ngsi-ld:Vehicle:B9211","error":{"type":"https://uri.etsi.org/ngsi-ld/errors/AlreadyExists","title":"Already exists"},"registrationId":"urn:ngsi-ld:ContextSourceRegistration:r1"}]}"#;
        let result: BatchOperationResult = serde_json::from_str(text).unwrap();
        assert_eq!(
            result,
            BatchOperationResult {
                success: vec![iri!("urn:ngsi-ld:Vehicle:A4567").into()],
                errors: vec![BatchEntityError {
                    entity_id: iri!("urn:ngsi-ld:Vehicle:B9211").into(),
                    error: ProblemDetails {
                        error_type: Some(ErrorType::AlreadyExists),
                        title: Some("Already exists".to_owned()),
                        ..ProblemDetails::default()
                    },
                    registration_id: Some("urn:ngsi-ld:ContextSourceRegistration:r1".parse().unwrap()),
                }],
            }
        );
        assert_eq!(serde_json::to_string(&result).unwrap(), text);
        assert_eq!(result.render().to_string(), text);
    }

    #[test]
    fn a_result_missing_both_lists_reads_as_empty() {
        assert_eq!(serde_json::from_str::<BatchOperationResult>("{}").unwrap(), BatchOperationResult::default());
    }

    #[test]
    fn an_error_whose_entity_id_is_not_a_uri_is_rejected() {
        assert!(serde_json::from_str::<BatchOperationResult>(r#"{"success":["not a uri"],"errors":[]}"#).is_err());
    }
}
