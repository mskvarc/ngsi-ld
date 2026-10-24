//! Problem details: the body of every NGSI-LD error response (NGSI-LD clauses 5.5.2, 5.5.3 and 6.3.3; IETF RFC
//! 7807).

use crate::model::RawJson;
use bon::bon;
use iri_rs::{InvalidIri, IriRefBuf};
use jstrict::{Object, Value as Json};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// The type of an NGSI-LD error.
///
/// NGSI-LD clause 5.5.2, Table 5.5.2-1.
///
/// The twelve types of the table have their own variants; any other problem type a broker reports is kept as
/// [`ErrorType::Other`], as the IRI reference it was written as. RFC 7807 section 3.1 makes the type a URI reference,
/// so a relative one is kept as written, and brokers commonly define their own types, so an unknown type is not a
/// decoding error. A value read from a document is always the named variant when one exists, so matching on the
/// variants is reliable; constructing `Other` with the URL of a named type yields a value that does not compare
/// equal to it.
///
/// The enum is `#[non_exhaustive]` because later versions of the specification add error types.
///
/// ```
/// use ngsi_ld::model::ErrorType;
///
/// let known: ErrorType = "https://uri.etsi.org/ngsi-ld/errors/ResourceNotFound".parse()?;
/// assert_eq!(known, ErrorType::ResourceNotFound);
/// let other: ErrorType = "https://broker.example/errors/TenantNotFound".parse()?;
/// assert!(matches!(other, ErrorType::Other(_)));
/// let relative: ErrorType = "errors/TenantNotFound".parse()?;
/// assert_eq!(relative.to_string(), "errors/TenantNotFound");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, strum::Display, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
#[non_exhaustive]
pub enum ErrorType {
    /// The referred element already exists.
    #[strum(to_string = "https://uri.etsi.org/ngsi-ld/errors/AlreadyExists")]
    AlreadyExists,
    /// The request includes input data which does not meet the requirements of the operation.
    #[strum(to_string = "https://uri.etsi.org/ngsi-ld/errors/BadRequestData")]
    BadRequestData,
    /// The operation conflicts with the current state of the system.
    #[strum(to_string = "https://uri.etsi.org/ngsi-ld/errors/Conflict")]
    Conflict,
    /// There has been an error during the operation execution.
    #[strum(to_string = "https://uri.etsi.org/ngsi-ld/errors/InternalError")]
    InternalError,
    /// The request is syntactically invalid or includes wrong content.
    #[strum(to_string = "https://uri.etsi.org/ngsi-ld/errors/InvalidRequest")]
    InvalidRequest,
    /// A remote JSON-LD `@context` referenced in a request cannot be retrieved.
    #[strum(to_string = "https://uri.etsi.org/ngsi-ld/errors/LdContextNotAvailable")]
    LdContextNotAvailable,
    /// The implementation does not support multiple tenants.
    #[strum(to_string = "https://uri.etsi.org/ngsi-ld/errors/NoMultiTenantSupport")]
    NoMultiTenantSupport,
    /// The addressed tenant does not exist.
    #[strum(to_string = "https://uri.etsi.org/ngsi-ld/errors/NonexistentTenant")]
    NonexistentTenant,
    /// The operation is not supported.
    #[strum(to_string = "https://uri.etsi.org/ngsi-ld/errors/OperationNotSupported")]
    OperationNotSupported,
    /// The referred resource has not been found.
    #[strum(to_string = "https://uri.etsi.org/ngsi-ld/errors/ResourceNotFound")]
    ResourceNotFound,
    /// The query is too complex and cannot be resolved.
    #[strum(to_string = "https://uri.etsi.org/ngsi-ld/errors/TooComplexQuery")]
    TooComplexQuery,
    /// The query produces so many results that it can exhaust client or server resources.
    #[strum(to_string = "https://uri.etsi.org/ngsi-ld/errors/TooManyResults")]
    TooManyResults,
    /// A problem type outside Table 5.5.2-1, absolute or relative.
    #[strum(to_string = "{0}")]
    Other(IriRefBuf),
}

impl ErrorType {
    /// The types of Table 5.5.2-1, each written as its URL by `Display`.
    const NAMED: [Self; 12] = [
        Self::AlreadyExists,
        Self::BadRequestData,
        Self::Conflict,
        Self::InternalError,
        Self::InvalidRequest,
        Self::LdContextNotAvailable,
        Self::NoMultiTenantSupport,
        Self::NonexistentTenant,
        Self::OperationNotSupported,
        Self::ResourceNotFound,
        Self::TooComplexQuery,
        Self::TooManyResults,
    ];
}

/// The named type whose URL this is, or [`ErrorType::Other`].
impl From<IriRefBuf> for ErrorType {
    fn from(reference: IriRefBuf) -> Self {
        Self::NAMED
            .into_iter()
            .find(|named| named.to_string() == reference.as_str())
            .unwrap_or(Self::Other(reference))
    }
}

// The named variants are recognised from the URL, which `strum::EnumString` cannot do for the `Other` fallback.
impl FromStr for ErrorType {
    type Err = InvalidIri<String>;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        IriRefBuf::new(text.to_owned()).map(Self::from)
    }
}

// Serde reads the type through an owned string.
impl TryFrom<String> for ErrorType {
    type Error = InvalidIri<String>;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

// Serde writes the type through an owned string.
impl From<ErrorType> for String {
    fn from(error_type: ErrorType) -> Self {
        error_type.to_string()
    }
}

/// The body of an error response.
///
/// NGSI-LD clause 5.5.3, IETF RFC 7807 section 3.1.
///
/// Clause 5.5.3 requires `type`, `title` and `detail`; brokers do not always send all three, so every member is
/// optional here and a missing one is no reason to lose the rest. A missing `type` means `about:blank` (RFC 7807
/// section 4.2). Members outside RFC 7807 section 3.1 are ignored.
///
/// Reading is lenient, member by member: a member that does not have its typed shape (a `type` or `instance` that
/// is not an IRI reference, a `title` that is not a string, a member written twice) is left out instead of failing
/// the whole problem, and a `status` written as a numeric string is read as the number. An error response keeps
/// its body as received beside the problem (`ResponseError::Problem`), so a member left out here is not lost.
///
/// ```
/// use ngsi_ld::model::{ErrorType, ProblemDetails};
///
/// let problem: ProblemDetails = serde_json::from_str(
///     r#"{
///       "type": "https://uri.etsi.org/ngsi-ld/errors/ResourceNotFound",
///       "title": "Resource not found.",
///       "detail": "urn:ngsi-ld:Device:widget001 was not found",
///       "status": 404,
///       "instance": "urn:ngsi-ld:Device:widget001"
///     }"#,
/// )?;
/// assert_eq!(problem.error_type, Some(ErrorType::ResourceNotFound));
/// assert_eq!(problem.status, Some(404));
/// # Ok::<(), anyhow::Error>(())
/// ```
///
/// The struct is `#[non_exhaustive]`, since RFC 9457 extension members may arrive as new fields. A problem to
/// report is built with [`ProblemDetails::builder`], which requires the `type`, `title` and `detail` clause 5.5.3
/// makes mandatory, and written with [`render`](Self::render):
///
/// ```
/// use iri_rs::{IriRef, iri};
/// use ngsi_ld::model::{ErrorType, ProblemDetails};
///
/// let problem = ProblemDetails::builder()
///     .error_type(ErrorType::ResourceNotFound)
///     .title("Resource not found.")
///     .detail("urn:ngsi-ld:Device:widget001 was not found")
///     .status(404)
///     .instance(IriRef::from(iri!("urn:ngsi-ld:Device:widget001")).into())
///     .build();
/// assert_eq!(
///     problem.render().to_string(),
///     concat!(
///         r#"{"type":"https://uri.etsi.org/ngsi-ld/errors"#,
///         r#"/ResourceNotFound","title":"Resource not found.","#,
///         r#""detail":"urn:ngsi-ld:Device:widget001 was not found","#,
///         r#""status":404,"instance":"urn:ngsi-ld:Device:widget001"}"#
///     )
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(from = "LenientProblem")]
#[non_exhaustive]
pub struct ProblemDetails {
    /// The problem type.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub error_type: Option<ErrorType>,
    /// A short summary of the problem.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// An explanation of this occurrence of the problem.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// The HTTP status code the broker gave.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    /// A reference to this occurrence of the problem, often the entity at fault.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance: Option<IriRefBuf>,
}

#[bon]
impl ProblemDetails {
    /// Starts the builder of a problem to report (clause 5.5.3): `type`, `title` and `detail` are required, the
    /// HTTP `status` and the `instance` at fault are optional.
    #[builder(finish_fn = build)]
    pub const fn new(
        error_type: ErrorType,
        #[builder(into)] title: String,
        #[builder(into)] detail: String,
        status: Option<u16>,
        instance: Option<IriRefBuf>,
    ) -> ProblemDetails {
        ProblemDetails {
            error_type: Some(error_type),
            title: Some(title),
            detail: Some(detail),
            status,
            instance,
        }
    }
}

impl ProblemDetails {
    /// Writes the problem as JSON, its members in the order `type`, `title`, `detail`, `status`, `instance`.
    #[must_use]
    pub fn render(&self) -> RawJson {
        RawJson::from_json(self.to_json())
    }

    /// Reads a problem leniently: every member that has its typed shape, and nothing else.
    pub(crate) fn from_json(json: &Json) -> ProblemDetails {
        let Some(object) = json.as_object() else {
            return ProblemDetails::default();
        };
        // A member written twice is ambiguous, so it is left out like a member out of shape.
        let member = |name: &str| object.get_unique(name).ok().flatten();
        let text = |name: &str| member(name).and_then(Json::as_str);
        ProblemDetails {
            error_type: text("type").and_then(|text| text.parse().ok()),
            title: text("title").map(str::to_owned),
            detail: text("detail").map(str::to_owned),
            status: member("status").and_then(status_code),
            instance: text("instance").and_then(|text| IriRefBuf::new(text.to_owned()).ok()),
        }
    }

    /// Writes the problem as a JSON object, its members in the order the struct declares them: `type`, `title`,
    /// `detail`, `status`, `instance`.
    pub(crate) fn to_json(&self) -> Json {
        let mut object = Object::new();
        if let Some(error_type) = &self.error_type {
            object.push("type".into(), Json::from(error_type.to_string()));
        }
        if let Some(title) = &self.title {
            object.push("title".into(), Json::from(title.as_str()));
        }
        if let Some(detail) = &self.detail {
            object.push("detail".into(), Json::from(detail.as_str()));
        }
        if let Some(status) = self.status {
            object.push("status".into(), Json::from(status));
        }
        if let Some(instance) = &self.instance {
            object.push("instance".into(), Json::from(instance.as_str()));
        }
        Json::Object(object)
    }
}

/// A status code written as a JSON number, or as a string holding one.
fn status_code(json: &Json) -> Option<u16> {
    let code = match json.as_number() {
        Some(number) => number.as_u64(),
        None => json.as_str()?.parse().ok(),
    };
    code.and_then(|code| u16::try_from(code).ok())
}

/// Any JSON, read as a problem by [`ProblemDetails::from_json`], so that serde reads problems as leniently as the
/// error decoder does (the problem of a 207 entry, clause 5.2.17).
#[derive(Deserialize)]
#[serde(transparent)]
struct LenientProblem(Json);

impl From<LenientProblem> for ProblemDetails {
    fn from(raw: LenientProblem) -> Self {
        Self::from_json(&raw.0)
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{ErrorType, ProblemDetails};
    use iri_rs::IriRefBuf;
    use pretty_assertions::assert_eq;

    #[test]
    fn every_type_of_table_5_5_2_1_reads_as_its_named_variant_and_writes_back_its_url() {
        for named in ErrorType::NAMED {
            let url = named.to_string();
            assert!(url.starts_with("https://uri.etsi.org/ngsi-ld/errors/"));
            assert_eq!(url.parse::<ErrorType>().unwrap(), named);
        }
    }

    #[test]
    fn an_unknown_problem_type_is_kept_as_other_with_its_url() {
        let other: ErrorType = "https://broker.example/errors/TenantNotFound".parse().unwrap();
        assert_eq!(other.to_string(), "https://broker.example/errors/TenantNotFound");
        assert!(matches!(other, ErrorType::Other(_)));
    }

    #[test]
    fn a_relative_problem_type_is_other_and_one_that_is_no_iri_reference_is_rejected() {
        assert!(matches!("ResourceNotFound".parse::<ErrorType>(), Ok(ErrorType::Other(_))));
        assert!("Resource Not Found".parse::<ErrorType>().is_err());
    }

    #[test]
    fn a_member_out_of_shape_is_left_out_and_the_others_are_read() {
        let problem: ProblemDetails =
            serde_json::from_str(r#"{"type": "Not Found", "title": ["a"], "detail": "gone", "status": "404", "instance": "/entities/a b"}"#).unwrap();
        assert_eq!(
            problem,
            ProblemDetails {
                detail: Some("gone".to_owned()),
                status: Some(404),
                ..ProblemDetails::default()
            }
        );
    }

    #[test]
    fn a_status_out_of_range_or_a_member_written_twice_is_left_out() {
        let problem = ProblemDetails::from_json(&r#"{"status": 70000, "title": "a", "title": "b"}"#.parse().unwrap());
        assert_eq!(problem, ProblemDetails::default());
    }

    #[test]
    fn the_clause_5_5_3_example_reads_with_its_instance() {
        let problem: ProblemDetails = serde_json::from_str(
            r#"{"type": "https://uri.etsi.org/ngsi-ld/errors/ResourceNotFound", "title": "Resource not found.", "detail": "urn:ngsi-ld:Device:widget001 was not found", "status": 404, "instance": "urn:ngsi-ld:Device:widget001"}"#,
        )
        .unwrap();
        assert_eq!(problem.error_type, Some(ErrorType::ResourceNotFound));
        assert_eq!(problem.title.as_deref(), Some("Resource not found."));
        assert_eq!(problem.detail.as_deref(), Some("urn:ngsi-ld:Device:widget001 was not found"));
        assert_eq!(problem.status, Some(404));
        assert_eq!(problem.instance.as_ref().map(IriRefBuf::as_str), Some("urn:ngsi-ld:Device:widget001"));
    }

    #[test]
    fn a_problem_missing_members_and_carrying_extensions_still_reads() {
        let problem: ProblemDetails = serde_json::from_str(r#"{"title": "Oops", "extra": {"x": 1}}"#).unwrap();
        assert_eq!(
            problem,
            ProblemDetails {
                title: Some("Oops".to_owned()),
                ..ProblemDetails::default()
            }
        );
    }

    #[test]
    fn a_built_problem_renders_the_members_of_clause_5_5_3_in_order() {
        let problem = ProblemDetails::builder()
            .error_type(ErrorType::BadRequestData)
            .title("Bad request data")
            .detail("type is not a query string parameter of a POST query")
            .build();
        assert_eq!(
            problem.render().to_string(),
            r#"{"type":"https://uri.etsi.org/ngsi-ld/errors/BadRequestData","title":"Bad request data","detail":"type is not a query string parameter of a POST query"}"#
        );
        let read: ProblemDetails = serde_json::from_str(&problem.render().to_string()).unwrap();
        assert_eq!(read, problem);
    }

    #[test]
    fn a_problem_round_trips_through_json() {
        let text = r#"{"type":"https://uri.etsi.org/ngsi-ld/errors/AlreadyExists","title":"Already exists","detail":"urn:ngsi-ld:Vehicle:A4567","status":409}"#;
        let problem: ProblemDetails = serde_json::from_str(text).unwrap();
        assert_eq!(serde_json::to_string(&problem).unwrap(), text);
    }
}
