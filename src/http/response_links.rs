//! The links a page of a query and a retrieval read from their `Link` header: the neighbouring pages of a query
//! (NGSI-LD clause 6.3.10) and the neighbouring pages of the attribute instances of a temporal result (ETSI TS 104 176
//! clause 6.4.7.3).

use crate::http::{
    ResponseContext,
    headers::{RelationType, link::header::read_line},
    response::Success,
};
use http::{HeaderValue, header::LINK};
use url::Url;

/// The neighbouring pages of the attribute instances of a temporal result (ETSI TS 104 176 clause 6.4.7.3).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct IntervalLinks {
    /// The first resolvable `intervalafter` link: the next instances in ascending order of their timestamps.
    pub(crate) after: Option<Url>,
    /// The first resolvable `intervalbefore` link: the next instances in descending order of their timestamps.
    pub(crate) before: Option<Url>,
}

/// The links of a response.
#[derive(Debug, Default)]
pub(crate) struct ResponseLinks {
    /// The first resolvable `next` link.
    pub(crate) next: Option<Url>,
    /// The first resolvable `prev` link.
    pub(crate) prev: Option<Url>,
    /// The first resolvable `first` link.
    pub(crate) first: Option<Url>,
    /// The first resolvable `last` link.
    pub(crate) last: Option<Url>,
    /// The `intervalafter` and `intervalbefore` links.
    pub(crate) intervals: IntervalLinks,
    /// The lines holding a link-value that could not be read or resolved.
    pub(crate) unread: Vec<HeaderValue>,
}

impl ResponseLinks {
    /// Reads every `Link` line, keeping the first target of each relation, resolved against the request URL.
    pub(crate) fn read(response: &Success, context: &ResponseContext) -> ResponseLinks {
        response.headers().get_all(LINK).iter().fold(ResponseLinks::default(), |mut links, line| {
            let read = read_line(line);
            let mut unresolved = 0_usize;
            for value in &read.values {
                for (rel, slot) in [
                    (RelationType::Next, &mut links.next),
                    (RelationType::Prev, &mut links.prev),
                    (RelationType::First, &mut links.first),
                    (RelationType::Last, &mut links.last),
                    (RelationType::IntervalAfter, &mut links.intervals.after),
                    (RelationType::IntervalBefore, &mut links.intervals.before),
                ] {
                    if slot.is_none() && value.has_rel(&rel) {
                        match context.resolve(value.target().as_str()) {
                            Some(url) => *slot = Some(url),
                            None => unresolved += 1,
                        }
                    }
                }
            }
            if read.unread + unresolved > 0 {
                // The header map keeps the line; the links outlive the response, so they keep their own handle.
                links.unread.push(line.clone());
            }
            links
        })
    }
}
