//! Reading a subscription fragment (NGSI-LD clauses 5.4 and 5.5.8): the members it sets, and the ones it deletes
//! with the NGSI-LD Null.

use crate::{
    model::{
        BuildError,
        DocumentError,
        Input,
        keyword::CONTEXT,
        member_change::read_change,
        reader::{boolean, context, date_time, identifier, object, once, string, unexpected, url, within},
    },
    query::{EntitySplitting, Locality},
    seal::Seal,
    subscription::{
        Activation,
        DocumentType,
        Problem,
        ReadErrorKind,
        SubscriptionFragment,
        SubscriptionId,
        SubscriptionKind,
        SubscriptionMember,
        kind::sealed::KindChanges,
        member::SubscriptionMandatoryMember,
        reader::{
            Emptiness,
            comma_separated_names,
            conformance_version,
            dataset_id_list,
            error,
            expect_type,
            geo_query,
            language,
            names,
            notification_params::read_notification_params,
            notification_triggers,
            positive,
            selectors,
        },
    },
};
use jstrict::{Value as Json, object::Entry};

/// Reads a subscription fragment of kind `K`. `type` may be left out where the operation names the target (clause
/// 5.4); an `id` is checked and not kept, as is the `@context`, which belongs to the binding (clause 6.3.5). The
/// `notification` replaces the whole member, so it is read in full and cannot be the Null.
pub(crate) fn read_subscription_fragment<K: SubscriptionKind>(json: Json) -> Result<SubscriptionFragment<K>, DocumentError<ReadErrorKind<Problem>>> {
    let mut fragment = SubscriptionFragment::<K>::default();
    let mut id: Option<SubscriptionId> = None;
    let mut typed = None;
    let mut fragment_context = None;
    for Entry { key, value } in object(json)? {
        let key = key.as_str();
        if let Ok(member) = key.parse::<SubscriptionMember>() {
            read_optional(&mut fragment, member, value)?;
        } else if let Ok(mandatory) = key.parse::<SubscriptionMandatoryMember>() {
            let key = mandatory.into();
            match mandatory {
                SubscriptionMandatoryMember::Id => once(&mut id, key, within(key, identifier(value, key))?)?,
                SubscriptionMandatoryMember::Type => once(&mut typed, key, within(key, expect_type(value, DocumentType::Subscription))?)?,
                SubscriptionMandatoryMember::Notification => once(&mut fragment.notification, key, within(key, read_notification_params::<Input>(value))?)?,
            }
        } else if key == CONTEXT {
            once(&mut fragment_context, key, within(key, context(value))?)?;
        } else {
            return Err(unexpected(key));
        }
    }
    let mut problems = Vec::new();
    fragment.check_pairs(&mut problems);
    if let Some(notification) = &fragment.notification {
        problems.extend(notification.rule_problems());
    }
    BuildError::finish(problems, fragment).map_err(|source| error(ReadErrorKind::Invalid { source }))
}

/// Reads the change to an optional member into `fragment`.
fn read_optional<K: SubscriptionKind>(
    fragment: &mut SubscriptionFragment<K>,
    member: SubscriptionMember,
    value: Json,
) -> Result<(), DocumentError<ReadErrorKind<Problem>>> {
    let key = member.into();
    match member {
        SubscriptionMember::SubscriptionName => once(&mut fragment.name, key, within(key, read_change(value, |value| string(value, key)))?),
        SubscriptionMember::Description => once(&mut fragment.description, key, within(key, read_change(value, |value| string(value, key)))?),
        SubscriptionMember::Entities => once(&mut fragment.entities, key, within(key, read_change(value, selectors))?),
        SubscriptionMember::WatchedAttributes => once(
            &mut fragment.watched_attributes,
            key,
            within(key, read_change(value, |value| names(value, key, Emptiness::Refused)))?,
        ),
        SubscriptionMember::NotificationTrigger => once(
            &mut fragment.notification_triggers,
            key,
            within(key, read_change(value, |value| notification_triggers(value, key)))?,
        ),
        SubscriptionMember::TimeInterval => once(&mut fragment.time_interval, key, within(key, read_change(value, |value| positive(value, key)))?),
        SubscriptionMember::Throttling => once(&mut fragment.throttling, key, within(key, read_change(value, |value| positive(value, key)))?),
        SubscriptionMember::Q => once(&mut fragment.q, key, within(key, read_change(value, |value| language(value, key)))?),
        SubscriptionMember::GeoQ => once(&mut fragment.geo_q, key, within(key, read_change(value, geo_query))?),
        SubscriptionMember::ScopeQ => once(&mut fragment.scope_q, key, within(key, read_change(value, |value| language(value, key)))?),
        // A member of the other kind of subscription is not defined for this one.
        SubscriptionMember::TemporalQ | SubscriptionMember::Csf => {
            if fragment.kind_members.read(Seal::new(), member, value)?.is_some() {
                return Err(unexpected(key));
            }
            Ok(())
        }
        SubscriptionMember::DatasetId => once(
            &mut fragment.dataset_ids,
            key,
            within(key, read_change(value, |value| dataset_id_list(value, key)))?,
        ),
        SubscriptionMember::Lang => once(&mut fragment.lang, key, within(key, read_change(value, |value| language(value, key)))?),
        SubscriptionMember::ExpandValues => once(
            &mut fragment.expand_values,
            key,
            within(key, read_change(value, |value| comma_separated_names(value, key)))?,
        ),
        SubscriptionMember::JsonKeys => once(
            &mut fragment.json_keys,
            key,
            within(key, read_change(value, |value| comma_separated_names(value, key)))?,
        ),
        SubscriptionMember::LocalOnly => once(
            &mut fragment.locality,
            key,
            within(key, read_change(value, |value| boolean(&value, key).map(Locality::from_flag)))?,
        ),
        SubscriptionMember::SplitEntities => once(
            &mut fragment.split_entities,
            key,
            within(key, read_change(value, |value| boolean(&value, key).map(EntitySplitting::from_flag)))?,
        ),
        SubscriptionMember::IsActive => once(
            &mut fragment.activation,
            key,
            within(key, read_change(value, |value| boolean(&value, key).map(Activation::from_flag)))?,
        ),
        SubscriptionMember::ExpiresAt => once(&mut fragment.expires_at, key, within(key, read_change(value, |value| date_time(value, key)))?),
        SubscriptionMember::JsonldContext => once(&mut fragment.jsonld_context, key, within(key, read_change(value, |value| url(value, key)))?),
        SubscriptionMember::NgsildConformance => once(
            &mut fragment.conformance,
            key,
            within(key, read_change(value, |value| conformance_version(value, key)))?,
        ),
    }
}
