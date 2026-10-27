//! Why a subscription, its notification parameters, an endpoint or a fragment of a subscription could not be built,
//! and the rules of NGSI-LD Tables 5.2.12-1, 5.2.14.1-1 and 5.2.15-1 a read document breaks.

use crate::{
    model::MemberProblem,
    subscription::{NotificationTriggersError, SubscriptionField, SubscriptionMember},
};

/// One rule a subscription document breaks, or one invalid input to its builders.
///
/// The builders take names and URLs as typed values or as text validated when they finish, so the common case
/// reads in one line; each problem is recorded and `build()` returns them all at once as a
/// [`BuildError`](crate::model::BuildError) of [`Problem`].
///
/// The enum is `#[non_exhaustive]`: later versions of the specification add members and restrictions.
///
/// ```
/// use ngsi_ld::{
///     model::MemberProblem,
///     subscription::{Endpoint, EndpointMember, Problem},
/// };
///
/// let error = Endpoint::builder("not a url").timeout(0).build().err();
/// let problems = error.as_ref().map(|error| error.problems());
/// let Some(
///     [
///         Problem::Member(MemberProblem::InvalidUrl { .. }),
///         Problem::Member(MemberProblem::NotPositive { member, .. }),
///     ],
/// ) = problems
/// else {
///     anyhow::bail!("expected an invalid URL and a timeout that is not positive");
/// };
/// assert!(*member == EndpointMember::Timeout.into());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Problem {
    /// A member is invalid, empty, missing, or both set and deleted.
    #[error(transparent)]
    Member(#[from] MemberProblem<SubscriptionField>),
    /// The notification triggers are empty or repeat a trigger (Table 5.2.12-1).
    #[error("invalid notification triggers")]
    Triggers {
        /// Why.
        #[source]
        source: NotificationTriggersError,
    },
    /// Neither `entities` nor `watchedAttributes` is given, and the subscription is not local (Table 5.2.12-1).
    #[error("a subscription needs entities or watched attributes unless it is local")]
    NoSubscribedEntities,
    /// `timeInterval` is given without `entities`, and the subscription is not local (Table 5.2.12-1).
    #[error("a periodic subscription needs entities unless it is local")]
    TimeIntervalWithoutEntities,
    /// `watchedAttributes` and `timeInterval` are both given (Table 5.2.12-1).
    #[error("a subscription cannot both watch attributes and notify periodically")]
    WatchedAttributesWithTimeInterval,
    /// `throttling` and `timeInterval` are both given (Table 5.2.12-1).
    #[error("a periodic subscription cannot be throttled")]
    ThrottlingWithTimeInterval,
    /// An entity selector of every type (`*`) implies local scope, which `localOnly` false contradicts (Table
    /// 5.2.33-1).
    #[error("a selector of every entity type implies local scope, which localOnly false contradicts")]
    AllTypesNotLocal,
    /// `showChanges` is true with the simplified format (Table 5.2.14.1-1).
    #[error("previous values cannot be shown in the simplified format")]
    ShowChangesWithSimplified,
    /// A fragment of an entity subscription deletes `temporalQ`, which only a Context Source Registration Subscription
    /// has (Table 5.2.12-1).
    #[error("`{member}` is not a member of this kind of subscription")]
    MemberOfOtherKind {
        /// The member.
        member: SubscriptionMember,
    },
}
