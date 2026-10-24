//! The builder of a registration a client sends (NGSI-LD clauses 5.2.9 and 5.9.2).

use crate::{
    model::{BuildError, Context, ContextSourceAlias, DatasetId, DateTime, Duration, Input, OneOrMany, Scope, Tenant, UrlDraft, Value, geometry::Geometry},
    registration::{
        CSourceRegistration,
        ContextSourceInfoDraft,
        ContextSourcePropertyNameDraft,
        Problem,
        RegistrationField,
        RegistrationId,
        RegistrationInfo,
        RegistrationManagementInfo,
        RegistrationMember,
        RegistrationMode,
        SupportedOperation,
        TimeInterval,
        context_source_info,
        property,
        rules,
    },
};
use bon::bon;

#[bon]
impl CSourceRegistration<Input> {
    /// Starts a builder from the endpoint of the context source. `information` is required; every other member is
    /// optional, `context_source_info` adds one pair and `property` one Context Source Property (clause 4.9) each
    /// time it is called, and every problem is reported by `build()`.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing an endpoint that is not a URL, an empty `information`, an empty
    /// name or description, a `contextSourceInfo` value its key does not allow (clause 4.3.6.6), a Context Source
    /// Property name that is invalid or given twice, and the rules of the `auxiliary` and `exclusive` modes
    /// (clauses 4.3.6.2, 4.3.6.3 and 5.9.2.4).
    #[builder]
    pub fn new(
        #[builder(start_fn, into)] endpoint: UrlDraft,
        #[builder(field)] context_source_info: Vec<ContextSourceInfoDraft>,
        #[builder(field)] properties: Vec<(ContextSourcePropertyNameDraft, Value)>,
        #[builder(with = |information: impl IntoIterator<Item = RegistrationInfo>| information.into_iter().collect())] information: Vec<RegistrationInfo>,
        #[builder(into)] id: Option<RegistrationId>,
        #[builder(into)] registration_name: Option<String>,
        #[builder(into)] description: Option<String>,
        context_source_alias: Option<ContextSourceAlias>,
        mode: Option<RegistrationMode>,
        #[builder(with = |operations: impl IntoIterator<Item = impl Into<SupportedOperation>>| operations.into_iter().map(Into::into).collect())]
        operations: Option<Vec<SupportedOperation>>,
        management: Option<RegistrationManagementInfo>,
        tenant: Option<Tenant>,
        #[builder(into)] scope: Option<OneOrMany<Scope>>,
        #[builder(with = |ids: impl IntoIterator<Item = DatasetId>| ids.into_iter().collect())] dataset_ids: Option<Vec<DatasetId>>,
        #[builder(into)] location: Option<Geometry>,
        #[builder(into)] observation_space: Option<Geometry>,
        #[builder(into)] operation_space: Option<Geometry>,
        observation_interval: Option<TimeInterval>,
        management_interval: Option<TimeInterval>,
        expires_at: Option<DateTime>,
        refresh_rate: Option<Duration>,
        context: Option<Context>,
    ) -> Result<Self, BuildError<Problem>> {
        let mut problems = rules::member_problems(&rules::Members {
            information: &information,
            mode,
            operations: operations.as_deref(),
            texts: [
                (RegistrationMember::RegistrationName, registration_name.as_deref()),
                (RegistrationMember::Description, description.as_deref()),
            ],
        });
        // No pair added is no `contextSourceInfo` member.
        let context_source_info = (!context_source_info.is_empty()).then(|| context_source_info::resolve(context_source_info, &mut problems));
        let properties = property::resolve(properties, &mut problems);
        let registration = endpoint.resolve(RegistrationField::Endpoint, &mut problems).map(|endpoint| Self {
            id,
            name: registration_name,
            description,
            information,
            endpoint,
            context_source_info,
            context_source_alias,
            mode,
            operations,
            management,
            tenant,
            scope,
            dataset_ids,
            location,
            observation_space,
            operation_space,
            observation_interval,
            management_interval,
            expires_at,
            refresh_rate,
            properties,
            output: (),
            context,
        });
        BuildError::finish_partial(problems, registration)
    }
}

impl<S: c_source_registration_builder::State> CSourceRegistrationBuilder<S> {
    /// Adds a pair the broker conveys when it contacts the context source (`contextSourceInfo`, clause 4.3.6.5): a
    /// [`ContextSourceInfo`](crate::registration::ContextSourceInfo), or a key and a value
    /// read when the registration is built.
    pub fn context_source_info(mut self, info: impl Into<ContextSourceInfoDraft>) -> Self {
        self.context_source_info.push(info.into());
        self
    }

    /// Adds a Context Source Property (clause 4.9), a member that describes the context source and that a context
    /// source filter (`csf`) targets: a
    /// [`ContextSourcePropertyName`](crate::registration::ContextSourcePropertyName), or text read when the
    /// registration is built, and any NGSI-LD Value (clause 3.1). The properties are written in the order they are
    /// added.
    pub fn property(mut self, name: impl Into<ContextSourcePropertyNameDraft>, value: impl Into<Value>) -> Self {
        self.properties.push((name.into(), value.into()));
        self
    }
}
