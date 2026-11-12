//! Requests for the pages of attribute instances a Retrieve Temporal Evolution answer links (ETSI TS 104 176 clause
//! 6.4.7.3).

use crate::{
    http::{
        Endpoint,
        RequestError,
        ResponseContext,
        ResponseError,
        Retrieved,
        headers::NgsiLdEntityMap,
        operation::{
            Aggregated,
            NormalizedTemporal,
            Operation,
            SimplifiedTemporal,
            sealed::{BuildsRequest, Sealed},
            temporal::decode_retrieved,
        },
        request::{RequestParts, Target, scope::RequestScope},
    },
    model::{
        Complete,
        NgsiLdMediaType,
        Output,
        Projected,
        evolution::{AggregatedTemporalEntity, SimplifiedTemporalEntity, TemporalEntity},
    },
};
use bytes::Bytes;
use http::{Method, Request, Response};
use std::marker::PhantomData;
use url::Url;

/// Requests a page of attribute instances a Retrieve Temporal Evolution answer linked, read as a [`Retrieved`] `T`.
///
/// Its request is a `GET` of the `intervalafter` or `intervalbefore` link (ETSI TS 104 176 clause 6.4.7.3), which
/// carries every parameter of the original request, its `offsetN` included; it asks for `application/json` and names
/// the entity map the answer reports, since the following page belongs to the same retrieval (clause 4.3.6.7). The
/// request is built on the tenant and snapshot it is sent with, which are those of the original request.
///
/// A [`TemporalEntity`] is read in the normalized representation, the only lossless one a temporal request can ask
/// for (Table 6.3.12-1).
///
/// ```
/// use bytes::Bytes;
/// use http::Request;
/// use ngsi_ld::{
///     http::{Endpoint, Retrieved, operation::Operation},
///     model::{Output, evolution::TemporalEntity},
/// };
///
/// /// The request for the instances after those of `answer`, if the broker linked them.
/// fn later_instances(
///     answer: &Retrieved<TemporalEntity<Output>>,
///     endpoint: &Endpoint,
/// ) -> anyhow::Result<Option<Request<Bytes>>> {
///     let Some(later) = answer.interval_after_request() else {
///         return Ok(None);
///     };
///     Ok(Some(later.to_request(endpoint)?))
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntervalRequest<'a, T> {
    /// The link.
    url: &'a Url,
    /// The entity map the answer reports, if it reports one.
    entity_map: Option<&'a NgsiLdEntityMap>,
    /// The item type.
    item: PhantomData<fn() -> T>,
}

impl<T> Retrieved<T> {
    /// The request for the next page of attribute instances in ascending order of their timestamps, if the broker
    /// linked it (`intervalafter`).
    #[must_use]
    pub fn interval_after_request(&self) -> Option<IntervalRequest<'_, T>> {
        self.interval_at(self.interval_after())
    }

    /// The request for the next page of attribute instances in descending order of their timestamps, if the broker
    /// linked it (`intervalbefore`).
    #[must_use]
    pub fn interval_before_request(&self) -> Option<IntervalRequest<'_, T>> {
        self.interval_at(self.interval_before())
    }

    /// The request for the link `url`, naming the entity map this answer reports.
    fn interval_at<'a>(&'a self, url: Option<&'a Url>) -> Option<IntervalRequest<'a, T>> {
        url.map(|url| IntervalRequest {
            url,
            entity_map: self.entity_map().valid(),
            item: PhantomData,
        })
    }
}

impl<T> Sealed for IntervalRequest<'_, T> {}

impl<T> IntervalRequest<'_, T> {
    /// The link requested.
    #[must_use]
    pub const fn url(&self) -> &Url {
        self.url
    }

    /// The request: a GET of the link, asking for JSON, naming the entity map.
    fn parts(&self) -> RequestParts {
        RequestParts {
            method: Method::GET,
            // The parts own their target; a URL is small next to the request it becomes.
            target: Target::Url(self.url.clone()),
            body: None,
            accept: Some(NgsiLdMediaType::Json),
            entity_map: None,
        }
        .with_entity_map(self.entity_map)
    }
}

/// Implements [`Operation`] and [`BuildsRequest`] for the pages of instances of `$item`, read in `$format` with the
/// completeness `$completeness`.
macro_rules! interval_operation {
    ($item:ty, $completeness:ty, $format:expr) => {
        impl Operation for IntervalRequest<'_, $item> {
            type Outcome = Retrieved<$item>;

            fn decode(&self, context: &ResponseContext, response: Response<Bytes>) -> Result<Retrieved<$item>, ResponseError> {
                decode_retrieved::<$completeness, _>(context, response, $format)
            }
        }

        impl BuildsRequest for IntervalRequest<'_, $item> {
            fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
                self.parts().into_request(endpoint, scope)
            }
        }
    };
}

interval_operation!(TemporalEntity<Output>, Complete, NormalizedTemporal);
interval_operation!(TemporalEntity<Output, Projected>, Projected, NormalizedTemporal);
interval_operation!(SimplifiedTemporalEntity, Complete, SimplifiedTemporal);
interval_operation!(SimplifiedTemporalEntity<Projected>, Projected, SimplifiedTemporal);
interval_operation!(AggregatedTemporalEntity, Complete, Aggregated);
interval_operation!(AggregatedTemporalEntity<Projected>, Projected, Aggregated);
