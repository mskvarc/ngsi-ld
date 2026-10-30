//! The requests of the temporal operations (clauses 6.18 to 6.22 and 6.24).

use crate::{
    fixtures::{endpoint, endpoint_with, header_value, json, link_of, response},
    links::{context_link, link_field, link_value},
    temporal_fixtures::{B_PATH, EVOLUTION, T1, T2, T3, WINDOW, b, body_text, evolution, instance_id, query_string, request, speed, time, window},
};
use headers::Header;
use http::{Method, header};
use jstrict::Unordered;
use ngsi_ld::{
    http::{
        ContextDelivery,
        Page,
        RequestError,
        ResponseContext,
        headers::{NgsiLdTenant, RelationType},
        operation::{
            AddTemporalAttributes,
            Aggregated,
            ArgumentError,
            CreateOrUpdateTemporalEvolution,
            DeleteAttributeInstance,
            DeleteTemporalAttribute,
            DeleteTemporalEvolution,
            EntityIdMember,
            EntityMapRequest,
            InstanceSelection,
            ModifyAttributeInstance,
            Operation,
            QueryMethod,
            QueryTemporalEvolution,
            ResultsCount,
            RetrieveTemporalEvolution,
            SimplifiedTemporal,
            SystemAttributes,
            TemporalSelection,
        },
    },
    model::{
        Attribute,
        DatasetId,
        EntityId,
        Input,
        Lossless,
        OneOrMany,
        Output,
        Projected,
        Property,
        Scope,
        Tenant,
        evolution::{InstanceSpelling, ScopeInstance, TemporalAttributes, TemporalEntity},
        geometry::{Position, Shape},
    },
    query::{
        EntitySelector,
        EntitySplitting,
        IdPattern,
        Locality,
        SelectorType,
        language_filter,
        ordering,
        projection::{self, EntityMember, Projection},
        q,
        temporal_query::{AggregationMethods, TemporalProperty, TemporalQuery},
    },
};
use pretty_assertions::assert_eq;
use std::num::NonZeroU64;

/// A retrieve of `B` with `selection`, no other parameter, in `format`, of the complete entity.
fn retrieve<'a, F>(id: &'a EntityId, selection: TemporalSelection<'a>, format: F) -> RetrieveTemporalEvolution<'a, F> {
    RetrieveTemporalEvolution::builder().id(id).selection(selection).format(format).build()
}

/// A retrieve of `B` with `selection`, no other parameter, in `format`, of the entity a projection may reduce.
fn retrieve_projected<'a, F>(id: &'a EntityId, selection: TemporalSelection<'a>, format: F) -> RetrieveTemporalEvolution<'a, F, Projected> {
    RetrieveTemporalEvolution::projected_builder()
        .id(id)
        .selection(selection)
        .format(format)
        .build()
}

/// A selection without a temporal query.
const fn unbounded(last_n: Option<NonZeroU64>, aggregation_methods: Option<&AggregationMethods>) -> TemporalSelection<'_> {
    TemporalSelection::Unbounded {
        time_property: None,
        last_n,
        aggregation_methods,
        aggregation_period: None,
    }
}

/// A GET query with `temporal_q`, of the types `entity_type`, and no other parameter, of complete entities.
fn query<'a, F>(entity_type: Option<&'a SelectorType>, temporal_q: &'a TemporalQuery, format: F) -> QueryTemporalEvolution<'a, F> {
    QueryTemporalEvolution::builder()
        .maybe_entity_type(entity_type)
        .temporal_q(temporal_q)
        .format(format)
        .build()
}

/// `Vehicle`.
fn vehicle_type() -> SelectorType {
    "Vehicle".parse().unwrap()
}

/// The request error of `operation`.
fn refusal(operation: &impl Operation) -> RequestError {
    operation.to_request(&endpoint()).unwrap_err()
}

#[test]
fn create_or_update_posts_the_evolution_with_the_json_headers_and_no_accept() {
    let entity = evolution();
    let request = request(&CreateOrUpdateTemporalEvolution::builder().entity(&entity).build());
    assert_eq!(request.method(), Method::POST);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/temporal/entities");
    assert_eq!(request.uri().query(), None);
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(header_value(&request, &header::CONTENT_LENGTH), Some(request.body().len().to_string().as_str()));
    assert_eq!(header_value(&request, &header::ACCEPT), None);
    assert_eq!(body_text(&request), EVOLUTION);
}

#[test]
fn a_concise_evolution_under_ld_json_body_carries_the_context_last() {
    let entity = evolution();
    let request = CreateOrUpdateTemporalEvolution::builder()
        .entity(&entity)
        .form(Lossless::Concise)
        .build()
        .to_request(&endpoint_with(ContextDelivery::LdJsonBody))
        .unwrap();
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/ld+json"));
    assert_eq!(link_of(&request), None);
    assert_eq!(
        body_text(&request),
        concat!(
            r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","speed":[{"value":120,"observedAt":"2018-08-01T12:03:00Z"},"#,
            r#"{"value":80,"observedAt":"2018-08-01T12:05:00Z"}],"@context":"https://example.org/context.jsonld"}"#
        )
    );
}

#[test]
fn create_or_update_writes_an_evolution_of_one_instance_as_an_object_when_asked() {
    let entity = TemporalEntity::<Input>::builder()
        .id(b())
        .entity_type("Vehicle")
        .property("speed", Property::builder(120).observed_at(time(T1)))
        .build()
        .unwrap();
    let single = request(
        &CreateOrUpdateTemporalEvolution::builder()
            .entity(&entity)
            .form(Lossless::Concise)
            .spelling(InstanceSpelling::SingleWhenOne)
            .build(),
    );
    assert_eq!(single.method(), Method::POST);
    assert_eq!(single.uri().path(), "/ngsi-ld/v1/temporal/entities");
    assert_eq!(single.uri().query(), None);
    assert_eq!(header_value(&single, &header::CONTENT_TYPE), Some("application/json"));
    assert_eq!(link_of(&single), Some(context_link()));
    assert_eq!(header_value(&single, &header::CONTENT_LENGTH), Some(single.body().len().to_string().as_str()));
    assert_eq!(
        body_text(&single),
        r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","speed":{"value":120,"observedAt":"2018-08-01T12:03:00Z"}}"#
    );
    let default = request(&CreateOrUpdateTemporalEvolution::builder().entity(&entity).form(Lossless::Concise).build());
    assert_eq!(
        body_text(&default),
        r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","speed":[{"value":120,"observedAt":"2018-08-01T12:03:00Z"}]}"#
    );
}

#[test]
fn create_or_update_writes_a_scope_of_one_instance_as_an_object_when_asked() {
    let entity = TemporalEntity::<Input>::builder()
        .id(b())
        .entity_type("Vehicle")
        .scope(ScopeInstance::new("/Madrid".parse::<Scope>().unwrap()).with_observed_at(time(T1)))
        .build()
        .unwrap();
    let single = request(
        &CreateOrUpdateTemporalEvolution::builder()
            .entity(&entity)
            .form(Lossless::Concise)
            .spelling(InstanceSpelling::SingleWhenOne)
            .build(),
    );
    assert_eq!(
        body_text(&single),
        r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","scope":{"value":"/Madrid","observedAt":"2018-08-01T12:03:00Z"}}"#
    );
    let default = request(&CreateOrUpdateTemporalEvolution::builder().entity(&entity).form(Lossless::Concise).build());
    assert_eq!(
        body_text(&default),
        r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","scope":[{"value":"/Madrid","observedAt":"2018-08-01T12:03:00Z"}]}"#
    );
}

#[test]
fn add_attributes_writes_one_instance_as_an_array() {
    let mut attributes = TemporalAttributes::<Input>::new();
    attributes
        .insert(speed(), Property::builder(70).observed_at(time(T3)).build().unwrap().into())
        .unwrap();
    let id = b();
    let request = request(&AddTemporalAttributes::builder().id(&id).attributes(&attributes).form(Lossless::Concise).build());
    assert_eq!(request.method(), Method::POST);
    assert_eq!(request.uri().path(), format!("{B_PATH}/attrs"));
    assert_eq!(body_text(&request), r#"{"speed":[{"value":70,"observedAt":"2018-08-01T12:07:00Z"}]}"#);
}

#[test]
fn add_attributes_may_name_its_target_by_id_and_add_entity_types() {
    let mut attributes = TemporalAttributes::<Input>::new();
    attributes
        .insert(speed(), Property::builder(70).observed_at(time(T3)).build().unwrap().into())
        .unwrap();
    let id = b();
    let both_types = OneOrMany::try_from(vec!["Vehicle".parse().unwrap(), "Car".parse().unwrap()]).unwrap();
    let named = AddTemporalAttributes::builder()
        .id(&id)
        .attributes(&attributes)
        .form(Lossless::Concise)
        .id_member(EntityIdMember::Written)
        .entity_type(&both_types)
        .build();
    assert_eq!(
        body_text(&request(&named)),
        r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":["Vehicle","Car"],"speed":[{"value":70,"observedAt":"2018-08-01T12:07:00Z"}]}"#
    );
    let one = OneOrMany::one("Car".parse().unwrap());
    let retyped = AddTemporalAttributes {
        id_member: EntityIdMember::Omitted,
        entity_type: Some(&one),
        ..named
    };
    assert_eq!(
        body_text(&request(&retyped)),
        r#"{"type":"Car","speed":[{"value":70,"observedAt":"2018-08-01T12:07:00Z"}]}"#
    );
}

#[test]
fn delete_attribute_selects_the_default_instance_a_dataset_or_every_instance() {
    let id = b();
    let name = speed();
    let gps = "urn:ngsi-ld:dataset:gps".parse().unwrap();
    let delete = |instances| DeleteTemporalAttribute::builder().id(&id).name(&name).instances(instances).build();
    let default = request(&delete(InstanceSelection::Default));
    assert_eq!(default.method(), Method::DELETE);
    assert_eq!(default.uri().path(), format!("{B_PATH}/attrs/speed"));
    assert_eq!(default.uri().query(), None);
    assert_eq!(link_of(&default), Some(context_link()));
    assert_eq!(header_value(&default, &header::ACCEPT), None);
    assert!(default.body().is_empty());
    assert_eq!(
        query_string(&delete(InstanceSelection::Dataset(&gps))).as_deref(),
        Some("datasetId=urn%3Angsi-ld%3Adataset%3Agps")
    );
    assert_eq!(query_string(&delete(InstanceSelection::All)).as_deref(), Some("deleteAll=true"));
}

#[test]
fn delete_attribute_can_spell_out_delete_all_false_with_or_without_a_dataset() {
    let id = b();
    let name = speed();
    let gps = "urn:ngsi-ld:dataset:gps".parse().unwrap();
    let delete = |instances| DeleteTemporalAttribute::builder().id(&id).name(&name).instances(instances).build();
    assert_eq!(query_string(&delete(InstanceSelection::Selected(None))).as_deref(), Some("deleteAll=false"));
    assert_eq!(
        query_string(&delete(InstanceSelection::Selected(Some(&gps)))).as_deref(),
        Some("datasetId=urn%3Angsi-ld%3Adataset%3Agps&deleteAll=false")
    );
}

#[test]
fn modify_instance_patches_one_instance_as_the_only_element_of_an_array() {
    let id = b();
    let name = speed();
    let instance_id = instance_id();
    let instance: Attribute<Input> = Property::builder(90).observed_at(time(T2)).build().unwrap().into();
    let modify = |form| {
        ModifyAttributeInstance::builder()
            .id(&id)
            .name(&name)
            .instance_id(&instance_id)
            .instance(&instance)
            .form(form)
            .build()
    };
    let normalized = request(&modify(Lossless::Normalized));
    assert_eq!(normalized.method(), Method::PATCH);
    assert_eq!(normalized.uri().path(), format!("{B_PATH}/attrs/speed/urn:ngsi-ld:attribute:instance:1"));
    assert_eq!(
        body_text(&normalized),
        r#"{"speed":[{"type":"Property","value":90,"observedAt":"2018-08-01T12:05:00Z"}]}"#
    );
    assert_eq!(
        body_text(&request(&modify(Lossless::Concise))),
        r#"{"speed":[{"value":90,"observedAt":"2018-08-01T12:05:00Z"}]}"#
    );
}

#[test]
fn deleting_an_instance_or_the_evolution_sends_no_parameter_and_no_body() {
    let id = b();
    let name = speed();
    let instance_id = instance_id();
    let instance = request(&DeleteAttributeInstance {
        id: &id,
        name: &name,
        instance_id: &instance_id,
    });
    let evolution = request(&DeleteTemporalEvolution { id: &id });
    assert_eq!(instance.uri().path(), format!("{B_PATH}/attrs/speed/urn:ngsi-ld:attribute:instance:1"));
    assert_eq!(evolution.uri().path(), B_PATH);
    for request in [instance, evolution] {
        assert_eq!(request.method(), Method::DELETE);
        assert_eq!(request.uri().query(), None);
        assert!(request.body().is_empty());
        assert_eq!(link_of(&request), Some(context_link()));
    }
}

#[test]
fn retrieve_writes_the_temporal_query_and_asks_for_json() {
    let id = b();
    let window = window();
    let request = request(&retrieve(&id, TemporalSelection::Query(&window), Lossless::Normalized));
    assert_eq!(request.method(), Method::GET);
    assert_eq!(request.uri().path(), B_PATH);
    assert_eq!(request.uri().query(), Some(WINDOW));
    assert_eq!(header_value(&request, &header::ACCEPT), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
}

#[test]
fn retrieve_without_a_temporal_query_sends_last_n_alone() {
    let id = b();
    let operation = RetrieveTemporalEvolution {
        system_attributes: SystemAttributes::Include,
        ..retrieve(&id, unbounded(NonZeroU64::new(2), None), Lossless::Concise)
    };
    assert_eq!(query_string(&operation).as_deref(), Some("lastN=2&format=concise&options=sysAttrs"));
}

#[test]
fn retrieve_in_temporal_values_writes_its_selection_in_order() {
    let id = b();
    let after = TemporalQuery::after(time(T1)).with_time_property(TemporalProperty::ModifiedAt);
    let pick: Projection = projection::attributes(["speed"]).build().unwrap();
    let datasets: [DatasetId; 1] = ["@none".parse().unwrap()];
    let lang = language_filter::language("en").build().unwrap();
    let operation = RetrieveTemporalEvolution {
        pick: Some(&pick),
        dataset_ids: &datasets,
        lang: Some(&lang),
        entity_map: None,
        ..retrieve_projected(&id, TemporalSelection::Query(&after), SimplifiedTemporal)
    };
    assert_eq!(
        query_string(&operation).as_deref(),
        Some("pick=speed&datasetId=%40none&lang=en&timerel=after&timeAt=2018-08-01T12%3A03%3A00Z&timeproperty=modifiedAt&format=temporalValues")
    );
}

#[test]
fn an_aggregated_retrieve_needs_aggregation_methods() {
    let id = b();
    let methods: AggregationMethods = "max,avg".parse().unwrap();
    let aggregated = window().with_aggregation_methods(methods).with_aggregation_period("PT1M".parse().unwrap());
    assert_eq!(
        query_string(&retrieve(&id, TemporalSelection::Query(&aggregated), Aggregated)),
        Some(format!("{WINDOW}&aggrMethods=max,avg&aggrPeriodDuration=PT1M&format=aggregatedValues"))
    );
    let total: AggregationMethods = "totalCount".parse().unwrap();
    assert_eq!(
        query_string(&retrieve(&id, unbounded(None, Some(&total)), Aggregated)).as_deref(),
        Some("aggrMethods=totalCount&format=aggregatedValues")
    );
    assert!(matches!(
        refusal(&retrieve(&id, unbounded(None, None), Aggregated)),
        RequestError::Argument(ArgumentError::AggregationWithoutMethods)
    ));
}

#[test]
fn a_projection_removing_the_id_needs_a_projected_retrieve() {
    let id = b();
    let pick: Projection = projection::attributes(["speed"]).build().unwrap();
    let complete = RetrieveTemporalEvolution {
        pick: Some(&pick),
        ..retrieve(&id, unbounded(None, None), Lossless::Normalized)
    };
    assert!(matches!(
        refusal(&complete),
        RequestError::Argument(ArgumentError::ProjectionRemovesMember { member: EntityMember::Id })
    ));
    let projected = RetrieveTemporalEvolution {
        pick: Some(&pick),
        ..retrieve_projected(&id, unbounded(None, None), Lossless::Normalized)
    };
    assert_eq!(query_string(&projected).as_deref(), Some("pick=speed"));
}

#[test]
fn a_get_query_writes_the_selection_then_the_temporal_query_then_the_paging() {
    let entity_type = vehicle_type();
    let temporal_q = window().with_last_n(NonZeroU64::new(5).unwrap());
    let speeding = q::attr("speed").gt(50).build().unwrap();
    let operation = QueryTemporalEvolution {
        q: Some(&speeding),
        limit: Some(2),
        count: ResultsCount::Include,
        ..query(Some(&entity_type), &temporal_q, Lossless::Normalized)
    };
    let request = request(&operation);
    assert_eq!(request.method(), Method::GET);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/temporal/entities");
    assert_eq!(
        request.uri().query().map(str::to_owned),
        Some(format!("type=Vehicle&q=speed%3E50&{WINDOW}&lastN=5&limit=2&count=true"))
    );
    assert_eq!(header_value(&request, &header::ACCEPT), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
}

#[test]
fn a_get_query_writes_the_ordering_the_distribution_and_the_locality() {
    let entity_type = vehicle_type();
    let ids = [b(), "urn:ngsi-ld:Vehicle:A4567".parse().unwrap()];
    let after = TemporalQuery::after(time(T1));
    let by_id = ordering::descending("id").collation("de-u-co-phonebk").build().unwrap();
    let entity_map = EntityMapRequest::returned();
    let operation = QueryTemporalEvolution {
        ids: &ids,
        ordering: Some(&by_id),
        split_entities: EntitySplitting::Split,
        entity_map: Some(&entity_map),
        system_attributes: SystemAttributes::Include,
        offset: Some(4),
        locality: Locality::Local,
        ..query(Some(&entity_type), &after, Lossless::Concise)
    };
    assert_eq!(
        query_string(&operation).as_deref(),
        Some(concat!(
            "type=Vehicle&id=urn%3Angsi-ld%3AVehicle%3AB9211,urn%3Angsi-ld%3AVehicle%3AA4567",
            "&timerel=after&timeAt=2018-08-01T12%3A03%3A00Z&orderBy=id;desc&collation=de-u-co-phonebk",
            "&splitEntities=true&entityMap=true&format=concise&options=sysAttrs&offset=4&local=true"
        ))
    );
}

#[test]
fn a_post_query_carries_the_temporal_query_in_the_body() {
    let entity_type = vehicle_type();
    let temporal_q = window().with_last_n(NonZeroU64::new(5).unwrap());
    let speeding = q::attr("speed").gt(50).build().unwrap();
    let operation = QueryTemporalEvolution {
        q: Some(&speeding),
        limit: Some(10),
        method: QueryMethod::Post,
        ..query(Some(&entity_type), &temporal_q, SimplifiedTemporal)
    };
    let request = request(&operation);
    assert_eq!(request.method(), Method::POST);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/temporal/entityOperations/query");
    assert_eq!(request.uri().query(), Some("format=temporalValues&limit=10"));
    assert_eq!(
        Unordered(json(request.body())),
        Unordered(json(
            format!(
                r#"{{"type":"Query","entities":[{{"type":"Vehicle"}}],"q":"speed>50","temporalQ":{{"timerel":"between","timeAt":"{T1}","endTimeAt":"{T3}","lastN":5}}}}"#
            )
            .as_bytes()
        ))
    );
}

#[test]
fn a_post_query_carries_several_selectors_each_as_the_caller_spelled_it() {
    let temporal_q = window();
    let pattern: IdPattern = "^urn:ngsi-ld:Car:A.*$".parse().unwrap();
    let selectors = [
        EntitySelector::new("Vehicle".parse().unwrap()).with_ids(OneOrMany::try_from(vec![b()]).unwrap()),
        EntitySelector::new("Car".parse().unwrap())
            .with_ids(OneOrMany::one("urn:ngsi-ld:Car:A1".parse().unwrap()))
            .with_id_pattern(pattern),
        EntitySelector::all(),
    ];
    let operation = QueryTemporalEvolution {
        selectors: &selectors,
        method: QueryMethod::Post,
        ..query(None, &temporal_q, Lossless::Normalized)
    };
    let request = request(&operation);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/temporal/entityOperations/query");
    assert_eq!(request.uri().query(), None);
    assert_eq!(
        Unordered(json(request.body())),
        Unordered(json(
            format!(
                r#"{{"type":"Query","entities":[
                    {{"type":"Vehicle","id":["urn:ngsi-ld:Vehicle:B9211"]}},
                    {{"type":"Car","id":"urn:ngsi-ld:Car:A1","idPattern":"^urn:ngsi-ld:Car:A.*$"}},
                    {{"type":"*"}}
                ],"temporalQ":{{"timerel":"between","timeAt":"{T1}","endTimeAt":"{T3}"}}}}"#
            )
            .as_bytes()
        ))
    );
}

#[test]
fn a_get_temporal_query_carries_one_selector_given_alone_and_refuses_more() {
    let temporal_q = window();
    let one = [EntitySelector::new("Vehicle".parse().unwrap())];
    let alone = QueryTemporalEvolution {
        selectors: &one,
        ..query(None, &temporal_q, Lossless::Normalized)
    };
    assert_eq!(query_string(&alone), Some(format!("type=Vehicle&{WINDOW}")));
    let entity_type = vehicle_type();
    let beside_type = QueryTemporalEvolution {
        selectors: &one,
        ..query(Some(&entity_type), &temporal_q, Lossless::Normalized)
    };
    assert!(matches!(
        refusal(&beside_type),
        RequestError::Argument(ArgumentError::SeveralSelectorsInQueryString)
    ));
    let two = [EntitySelector::new("Car".parse().unwrap()), EntitySelector::new("Bus".parse().unwrap())];
    let two_selectors = QueryTemporalEvolution {
        selectors: &two,
        ..query(None, &temporal_q, Lossless::Normalized)
    };
    assert!(matches!(
        refusal(&two_selectors),
        RequestError::Argument(ArgumentError::SeveralSelectorsInQueryString)
    ));
    let posted = QueryTemporalEvolution {
        method: QueryMethod::Post,
        ..two_selectors
    };
    assert!(posted.to_request(&endpoint()).is_ok());
}

#[test]
fn an_aggregated_query_writes_its_methods() {
    let entity_type = vehicle_type();
    let temporal_q = window().with_aggregation_methods("max".parse().unwrap());
    assert_eq!(
        query_string(&query(Some(&entity_type), &temporal_q, Aggregated)),
        Some(format!("type=Vehicle&{WINDOW}&aggrMethods=max&format=aggregatedValues"))
    );
}

#[test]
fn a_temporal_query_refuses_what_the_specification_refuses() {
    let entity_type = vehicle_type();
    let window = window();
    assert!(matches!(
        refusal(&query(None, &window, Lossless::Normalized)),
        RequestError::Argument(ArgumentError::UnboundedQuery)
    ));
    let zero = QueryTemporalEvolution {
        limit: Some(0),
        ..query(Some(&entity_type), &window, Lossless::Normalized)
    };
    assert!(matches!(refusal(&zero), RequestError::Argument(ArgumentError::ZeroLimitWithoutCount)));
    let by_speed = ordering::by("speed").build().unwrap();
    let nearest = ordering::nearest("location")
        .origin(Shape::Point(Position::try_new(8.0, 40.0).unwrap()))
        .build()
        .unwrap();
    for ordering in [&by_speed, &nearest] {
        let ordered = QueryTemporalEvolution {
            ordering: Some(ordering),
            ..query(Some(&entity_type), &window, Lossless::Normalized)
        };
        assert!(matches!(refusal(&ordered), RequestError::Argument(ArgumentError::TemporalOrdering)));
    }
    assert!(matches!(
        refusal(&query(Some(&entity_type), &window, Aggregated)),
        RequestError::Argument(ArgumentError::AggregationWithoutMethods)
    ));
    let pick: Projection = projection::attributes(["speed"]).build().unwrap();
    let picking = QueryTemporalEvolution {
        pick: Some(&pick),
        ..query(Some(&entity_type), &window, Lossless::Normalized)
    };
    assert!(matches!(
        refusal(&picking),
        RequestError::Argument(ArgumentError::ProjectionRemovesMember { member: EntityMember::Id })
    ));
    let ids = [b()];
    let untyped = QueryTemporalEvolution {
        ids: &ids,
        q: Some(&q::attr("speed").exists().build().unwrap()),
        method: QueryMethod::Post,
        ..query(None, &window, Lossless::Normalized)
    };
    assert!(matches!(refusal(&untyped), RequestError::Argument(ArgumentError::SelectorWithoutType)));
}

#[test]
fn every_temporal_operation_names_the_tenant() {
    let tenant: Tenant = "t1".parse().unwrap();
    let entity = evolution();
    let id = b();
    let name = speed();
    let instance_id = instance_id();
    let instance: Attribute<Input> = Property::builder(90).build().unwrap().into();
    let attributes = TemporalAttributes::<Input>::new();
    let window = window();
    let entity_type = vehicle_type();
    let endpoint = endpoint().with_tenant(tenant);
    let requests = [
        CreateOrUpdateTemporalEvolution::builder().entity(&entity).build().to_request(&endpoint),
        AddTemporalAttributes::builder().id(&id).attributes(&attributes).build().to_request(&endpoint),
        DeleteTemporalAttribute::builder().id(&id).name(&name).build().to_request(&endpoint),
        ModifyAttributeInstance::builder()
            .id(&id)
            .name(&name)
            .instance_id(&instance_id)
            .instance(&instance)
            .build()
            .to_request(&endpoint),
        DeleteAttributeInstance {
            id: &id,
            name: &name,
            instance_id: &instance_id,
        }
        .to_request(&endpoint),
        DeleteTemporalEvolution { id: &id }.to_request(&endpoint),
        retrieve(&id, TemporalSelection::Query(&window), Lossless::Normalized).to_request(&endpoint),
        query(Some(&entity_type), &window, Lossless::Normalized).to_request(&endpoint),
    ];
    for request in requests {
        assert_eq!(header_value(&request.unwrap(), NgsiLdTenant::name()), Some("t1"));
    }
}

#[test]
fn the_next_page_of_a_temporal_query_keeps_its_tenant_and_asks_for_json() {
    let entity_type = vehicle_type();
    let after = TemporalQuery::after(time(T1));
    let tenant: Tenant = "t1".parse().unwrap();
    let operation = query(Some(&entity_type), &after, Lossless::Normalized);
    let sent = operation.to_request(&endpoint().with_tenant(tenant)).unwrap();
    let link = link_field([link_value(
        "/ngsi-ld/v1/temporal/entities?type=Vehicle&timerel=after&timeAt=2018-08-01T12%3A03%3A00Z&offset=1",
        RelationType::Next,
    )]);
    let page: Page<TemporalEntity<Output>> = operation
        .decode(&ResponseContext::new(&sent), response(200, &[(&header::LINK, &link)], "[]"))
        .unwrap();
    let next = page.next_page().unwrap().to_request(&endpoint()).unwrap();
    assert_eq!(
        next.uri().to_string(),
        "http://broker.test/ngsi-ld/v1/temporal/entities?type=Vehicle&timerel=after&timeAt=2018-08-01T12%3A03%3A00Z&offset=1"
    );
    assert_eq!(header_value(&next, &header::ACCEPT), Some("application/json"));
    assert_eq!(header_value(&next, NgsiLdTenant::name()), Some("t1"));
}
