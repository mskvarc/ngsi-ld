//! Retrieve Entity (clause 6.5.3.1), one method per representation family, per completeness and per join shape.

use crate::{
    client::{Answering, ClientError, Entities, Transport, arguments::built},
    http::{
        Retrieved,
        operation::{EntityMapRequest, Flattened, GeoJson, InlineJoin, RepresentationSpelling, RetrieveEntity, Simplified, SystemAttributes},
    },
    model::{AttributeName, Complete, DatasetId, Entity, EntityId, Feature, Lossless, Output, Projected, SimplifiedEntity, Tenant},
    query::{
        Locality,
        language_filter::{LanguageFilter, builders::LanguageFilterBuilder},
        projection::{Projection, builders::ProjectionBuilder},
        type_selection::{TypeSelection, builders::TypeSelectionBuilder},
    },
};
use bon::bon;
use std::num::NonZeroU32;

/// Generates the Retrieve Entity method for one representation, one join shape and one completeness.
macro_rules! retrieve_method {
    (
        $(#[doc = $doc:literal])*
        $name:ident, $body:ty,
        $format:ident => $format_value:expr, [$($format_parameter:tt)*],
        join => $join_value:expr, [$($join_parameter:tt)*],
        completeness => $completeness:expr
    ) => {
        #[bon]
        impl<T: Transport, M: Answering> Entities<'_, T, M> {
            $(#[doc = $doc])*
            #[builder(derive(Debug), finish_fn(name = send, doc {
                /// Sends the request and reads the response.
                ///
                /// # Errors
                ///
                /// Returns [`ClientError`] when an argument is invalid, the request is refused before sending, the
                /// broker answers with an error, or the transport fails.
            }))]
            pub async fn $name(
                &self,
                #[builder(start_fn)] id: &EntityId,
                #[builder(into)] entity_type: Option<TypeSelectionBuilder>,
                #[builder(default)] attrs: Vec<AttributeName>,
                #[builder(into)] pick: Option<ProjectionBuilder>,
                #[builder(into)] omit: Option<ProjectionBuilder>,
                #[builder(default)] dataset_ids: Vec<DatasetId>,
                #[builder(into)] lang: Option<LanguageFilterBuilder>,
                $($join_parameter)*
                join_level: Option<NonZeroU32>,
                #[builder(default)] contained_by: Vec<EntityId>,
                entity_map: Option<&EntityMapRequest>,
                #[builder(default)] system_attributes: SystemAttributes,
                #[builder(default)] spelling: RepresentationSpelling,
                $($format_parameter)*
                #[builder(default)] locality: Locality,
                tenant: Option<&Tenant>,
            ) -> Result<M::Answer<Retrieved<$body>>, ClientError<T::Error>> {
                let entity_type: Option<TypeSelection> = built(entity_type)?;
                let pick: Option<Projection> = built(pick)?;
                let omit: Option<Projection> = built(omit)?;
                let lang: Option<LanguageFilter> = built(lang)?;
                let $format = $format_value;
                let retrieve = RetrieveEntity {
                    id,
                    entity_type: entity_type.as_ref(),
                    attrs: &attrs,
                    pick: pick.as_ref(),
                    omit: omit.as_ref(),
                    dataset_ids: &dataset_ids,
                    lang: lang.as_ref(),
                    join: $join_value,
                    join_level,
                    contained_by: &contained_by,
                    entity_map,
                    system_attributes,
                    spelling,
                    format: $format,
                    completeness: $completeness,
                };
                self.route.send_with_locality(locality, &retrieve, tenant).await
            }
        }
    };
}

retrieve_method!(
    /// Retrieve Entity (clause 6.5.3.1), in normalized or concise representation.
    ///
    /// A `pick` or `omit` that removes `id` or `type` is refused before sending; [`retrieve_projected`] reads such
    /// an answer.
    ///
    /// [`retrieve_projected`]: Entities::retrieve_projected
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the entity does not exist, an argument is invalid, or the call fails.
    retrieve, Entity<Output>,
    format => form, [#[builder(default)] form: Lossless,],
    join => join, [join: Option<InlineJoin>,],
    completeness => Complete
);

retrieve_method!(
    /// Retrieve Entity (clause 6.5.3.1), in simplified representation (clause 4.5.4).
    ///
    /// A `pick` or `omit` that removes `id` or `type` is refused before sending; [`retrieve_projected_simplified`]
    /// reads such an answer.
    ///
    /// [`retrieve_projected_simplified`]: Entities::retrieve_projected_simplified
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the entity does not exist, an argument is invalid, or the call fails.
    retrieve_simplified, SimplifiedEntity,
    format => Simplified, [],
    join => join, [join: Option<InlineJoin>,],
    completeness => Complete
);

retrieve_method!(
    /// Retrieve Entity (clause 6.5.3.1) with a flattened join (`join=flat`, clause 4.5.23.3), in normalized or
    /// concise representation: the entity, followed by the entities its relationships target.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the entity does not exist, an argument is invalid, or the call fails.
    retrieve_flattened, Vec<Entity<Output>>,
    format => form, [#[builder(default)] form: Lossless,],
    join => Flattened, [],
    completeness => Complete
);

retrieve_method!(
    /// Retrieve Entity (clause 6.5.3.1) with a flattened join (`join=flat`, clause 4.5.23.3), in simplified
    /// representation (clause 4.5.4): the entity, followed by the entities its relationships target.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the entity does not exist, an argument is invalid, or the call fails.
    retrieve_flattened_simplified, Vec<SimplifiedEntity>,
    format => Simplified, [],
    join => Flattened, [],
    completeness => Complete
);

retrieve_method!(
    /// Retrieve Entity (clause 6.5.3.1) with a `pick` or `omit` that may remove `id` or `type` (clause 4.21), in
    /// normalized or concise representation.
    ///
    /// The projection also applies to linked entities (Table 6.5.3.1-1), which are read as projected entities.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the entity does not exist, an argument is invalid, or the call fails.
    retrieve_projected, Entity<Output, Projected>,
    format => form, [#[builder(default)] form: Lossless,],
    join => join, [join: Option<InlineJoin>,],
    completeness => Projected
);

retrieve_method!(
    /// Retrieve Entity (clause 6.5.3.1) with a `pick` or `omit` that may remove `id` or `type` (clause 4.21), in
    /// simplified representation (clause 4.5.4).
    ///
    /// The projection also applies to linked entities (Table 6.5.3.1-1), which are read as projected entities.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the entity does not exist, an argument is invalid, or the call fails.
    retrieve_projected_simplified, SimplifiedEntity<Projected>,
    format => Simplified, [],
    join => join, [join: Option<InlineJoin>,],
    completeness => Projected
);

retrieve_method!(
    /// Retrieve Entity (clause 6.5.3.1) with a flattened join (`join=flat`, clause 4.5.23.3) and a `pick` or
    /// `omit` that may remove `id` or `type` from every entity of the answer (clause 4.21), in normalized or
    /// concise representation.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the entity does not exist, an argument is invalid, or the call fails.
    retrieve_flattened_projected, Vec<Entity<Output, Projected>>,
    format => form, [#[builder(default)] form: Lossless,],
    join => Flattened, [],
    completeness => Projected
);

retrieve_method!(
    /// Retrieve Entity (clause 6.5.3.1) with a flattened join (`join=flat`, clause 4.5.23.3) and a `pick` or
    /// `omit` that may remove `id` or `type` from every entity of the answer (clause 4.21), in simplified
    /// representation (clause 4.5.4).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the entity does not exist, an argument is invalid, or the call fails.
    retrieve_flattened_projected_simplified, Vec<SimplifiedEntity<Projected>>,
    format => Simplified, [],
    join => Flattened, [],
    completeness => Projected
);

retrieve_method!(
    /// Retrieve Entity (clause 6.5.3.1) in `GeoJSON` representation (`Accept: application/geo+json`, clauses
    /// 4.5.16.2 and 6.3.15), its attributes in normalized or concise representation: a Feature whose geometry is
    /// the value of `geometry_property`, or of `location` when it is not set.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the entity does not exist, an argument is invalid, or the call fails.
    retrieve_geo_json, Feature<Entity<Output>>,
    format => GeoJson { form, geometry_property }, [#[builder(default)] form: Lossless, geometry_property: Option<&AttributeName>,],
    join => join, [join: Option<InlineJoin>,],
    completeness => Complete
);

retrieve_method!(
    /// Retrieve Entity (clause 6.5.3.1) in simplified `GeoJSON` representation (`Accept: application/geo+json`,
    /// clauses 4.5.17.1 and 6.3.15): a Feature whose geometry is the value of `geometry_property`, or of `location`
    /// when it is not set.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the entity does not exist, an argument is invalid, or the call fails.
    retrieve_geo_json_simplified, Feature<SimplifiedEntity>,
    format => GeoJson { form: Simplified, geometry_property }, [geometry_property: Option<&AttributeName>,],
    join => join, [join: Option<InlineJoin>,],
    completeness => Complete
);
