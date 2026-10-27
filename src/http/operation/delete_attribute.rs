//! Delete Attribute (NGSI-LD clauses 5.6.5 and 6.7.3.2).

use crate::{
    http::{
        Endpoint,
        QueryParameter,
        RequestError,
        ResponseContext,
        ResponseError,
        UpdateOutcome,
        operation::{
            Operation,
            sealed::{BuildsRequest, Sealed},
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
        response::classify,
    },
    model::{AttributeName, EntityId, Uri},
    query::type_selection::TypeSelection,
};
use bytes::Bytes;
use http::{Method, Request, Response};

pub(super) mod parameters;

/// Which instances of an attribute a deletion removes.
///
/// It selects the instances Delete Attribute removes (NGSI-LD clause 6.7.3.2), and the instances whose evolution Delete
/// Attribute from Temporal Evolution removes (clause 6.21.3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InstanceSelection<'a> {
    /// The default instance, the one without a dataset id.
    #[default]
    Default,
    /// The instance with this dataset id (`datasetId`).
    Dataset(&'a Uri),
    /// Every instance (`deleteAll=true`).
    All,
    /// Only one instance, with the flag spelled out (`deleteAll=false`): the instance with this dataset id, or the
    /// default instance when there is none. The request removes what [`Dataset`](Self::Dataset) or
    /// [`Default`](Self::Default) removes, since the flag is `false` by default (Tables 6.7.3.2-1 and 6.21.3.1-1).
    Selected(Option<&'a Uri>),
}

impl InstanceSelection<'_> {
    /// Adds `datasetId` or `deleteAll`, the parameters Delete Attribute and Delete Attribute from Temporal
    /// Evolution share (Tables 6.7.3.2-1 and 6.21.3.1-1).
    pub(crate) fn push_query(self, query: &mut QueryString) {
        match self {
            Self::Default => {}
            Self::Dataset(dataset_id) => query.push(QueryParameter::DatasetId, dataset_id),
            Self::All => query.push(QueryParameter::DeleteAll, true),
            Self::Selected(dataset_id) => {
                if let Some(dataset_id) = dataset_id {
                    query.push(QueryParameter::DatasetId, dataset_id);
                }
                query.push(QueryParameter::DeleteAll, false);
            }
        }
    }
}

/// Removes instances of an attribute.
///
/// Its request is `DELETE /entities/{entityId}/attrs/{attrId}` (NGSI-LD clause 6.7.3.2).
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{DeleteAttribute, InstanceSelection, Operation},
///     },
///     model::{AttributeName, EntityId},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
/// let speed: AttributeName = "speed".parse()?;
/// let every_instance = DeleteAttribute::builder()
///     .id(&id)
///     .name(&speed)
///     .instances(InstanceSelection::All)
///     .build();
/// let request = every_instance.to_request(&endpoint)?;
/// assert_eq!(request.method(), "DELETE");
/// assert_eq!(
///     request.uri(),
///     concat!(
///         "http://broker.test/ngsi-ld/v1/entities",
///         "/urn:ngsi-ld:Vehicle:A4567/attrs/speed?deleteAll=true"
///     )
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, bon::Builder)]
pub struct DeleteAttribute<'a> {
    /// The entity.
    pub id: &'a EntityId,
    /// The attribute.
    pub name: &'a AttributeName,
    /// The instances to remove.
    #[builder(default)]
    pub instances: InstanceSelection<'a>,
    /// Deletes only if the entity's type matches (clause 4.17).
    pub entity_type: Option<&'a TypeSelection>,
}

impl Sealed for DeleteAttribute<'_> {}

impl Operation for DeleteAttribute<'_> {
    type Outcome = UpdateOutcome;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<UpdateOutcome, ResponseError> {
        UpdateOutcome::decode(&classify(response)?)
    }
}

impl BuildsRequest for DeleteAttribute<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        let mut query = QueryString::default();
        self.instances.push_query(&mut query);
        query.push_type_selection(QueryParameter::Type, self.entity_type);
        RequestParts::resource(Method::DELETE, ResourcePath::attribute(self.id, self.name), query).into_request(endpoint, scope)
    }
}
