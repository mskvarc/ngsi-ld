//! The query strings of the write operations a broker forwards, read back in typed form: Append Attributes (Table
//! 6.6.3.1-1), Batch Entity Upsert and Update (clauses 6.15.3.1 and 6.16.3.1), Purge Entities (Table 6.4.3.3-1) and
//! Delete Attribute (Table 6.7.3.2-1). Each request the crate builds reads back as the parameters it was built from.

use crate::{fixtures::vehicle_id, temporal_fixtures::query_string};
use iri_rs::iri;
use ngsi_ld::{
    http::{
        QueryParameter,
        operation::{
            AppendAttributes,
            AppendAttributesParameters,
            BatchUpdate,
            BatchUpdateParameters,
            BatchUpsert,
            BatchUpsertParameters,
            DeleteAll,
            DeleteAttribute,
            DeleteAttributeParameters,
            InstanceSelection,
            Operation,
            Overwrite,
            ParameterError,
            PurgeEntities,
            PurgeEntitiesParameters,
            UpsertMode,
            WithLocality,
        },
    },
    model::{AttributeName, DatasetId, Entity, EntityFragment, EntityId},
    query::{IdPattern, Locality, SelectorType, geoquery::GeoQuery, q::Query, scope_query::ScopeQuery, type_selection::TypeSelection},
};
use pretty_assertions::assert_eq;

/// The query string of the request `operation` builds, empty when it has none.
fn sent(operation: &impl Operation) -> String {
    query_string(operation).unwrap_or_default()
}

#[test]
fn an_append_reads_back_its_overwrite_type_and_locality() {
    let id = vehicle_id();
    let fragment = EntityFragment::builder().property("speed", 1).build().unwrap();
    let vehicles: TypeSelection = "Vehicle|Car".parse().unwrap();
    let append = AppendAttributes::builder()
        .id(&id)
        .fragment(&fragment)
        .overwrite(Overwrite::Keep)
        .entity_type(&vehicles)
        .build();
    let local = WithLocality::new(Locality::Local, &append);
    let received = AppendAttributesParameters::parse(&sent(&local)).unwrap();
    assert_eq!(received.overwrite, Overwrite::Keep);
    assert_eq!(received.entity_type.as_ref(), Some(&vehicles));
    assert_eq!(received.locality, Locality::Local);
    let replace = AppendAttributes {
        overwrite: Overwrite::Replace,
        entity_type: None,
        ..append
    };
    let received = AppendAttributesParameters::parse(&sent(&replace)).unwrap();
    assert_eq!(received.overwrite, Overwrite::Replace);
    assert_eq!(received.entity_type, None);
    assert_eq!(received.locality, Locality::Unspecified);
}

#[test]
fn an_append_refuses_a_parameter_or_option_its_table_does_not_define() {
    assert!(matches!(
        AppendAttributesParameters::parse("options=update").unwrap_err(),
        ParameterError::UnknownToken { parameter: QueryParameter::Options, token } if token == "update"
    ));
    assert!(matches!(AppendAttributesParameters::parse("attrs=speed").unwrap_err(), ParameterError::Unknown { name } if name == "attrs"));
    assert!(matches!(
        AppendAttributesParameters::parse("options=noOverwrite&options=noOverwrite").unwrap_err(),
        ParameterError::Repeated {
            parameter: QueryParameter::Options
        }
    ));
}

#[test]
fn an_upsert_reads_back_its_mode() {
    let entities = [Entity::builder().id(iri!("urn:ngsi-ld:Vehicle:A4567")).entity_type("Vehicle").build().unwrap()];
    for mode in [UpsertMode::Unspecified, UpsertMode::Update, UpsertMode::Replace] {
        let upsert = BatchUpsert::builder().entities(&entities).mode(mode).build();
        assert_eq!(BatchUpsertParameters::parse(&sent(&upsert)).unwrap().mode, mode);
    }
    assert_eq!(BatchUpsertParameters::parse("options=replace").unwrap().mode, UpsertMode::Replace);
    assert_eq!(BatchUpsertParameters::parse("").unwrap().mode, UpsertMode::Unspecified);
    assert_eq!(
        BatchUpsertParameters::parse("options=update&local=false").unwrap().locality,
        Locality::Distributed
    );
    assert!(matches!(
        BatchUpsertParameters::parse("options=replace,update").unwrap_err(),
        ParameterError::UpsertModeTwice
    ));
    assert!(matches!(
        BatchUpsertParameters::parse("options=noOverwrite").unwrap_err(),
        ParameterError::UnknownToken { parameter: QueryParameter::Options, token } if token == "noOverwrite"
    ));
}

#[test]
fn a_batch_update_reads_back_its_overwrite() {
    let fragments = [EntityFragment::for_entity(iri!("urn:ngsi-ld:Vehicle:A4567"), "Vehicle")
        .property("speed", 1)
        .build()
        .unwrap()];
    for overwrite in [Overwrite::Keep, Overwrite::Replace] {
        let update = BatchUpdate::builder().fragments(&fragments).overwrite(overwrite).build();
        assert_eq!(BatchUpdateParameters::parse(&sent(&update)).unwrap().overwrite, overwrite);
    }
    assert!(matches!(BatchUpdateParameters::parse("type=Vehicle").unwrap_err(), ParameterError::Unknown { name } if name == "type"));
}

#[test]
fn a_purge_reads_back_every_parameter_it_sends() {
    let vehicles: SelectorType = "Vehicle".parse().unwrap();
    let ids = [vehicle_id(), EntityId::from(iri!("urn:ngsi-ld:Vehicle:B9211"))];
    let pattern: IdPattern = "^urn:ngsi-ld:Vehicle:.*$".parse().unwrap();
    let q: Query = "speed>50;brandName==\"Mercedes\"".parse().unwrap();
    let geo_q = GeoQuery::parse(r#"{"georel":"near;maxDistance==2000","geometry":"Point","coordinates":[8,40],"geoproperty":"observationSpace"}"#).unwrap();
    let scope_q: ScopeQuery = "/Madrid/#".parse().unwrap();
    let csf: Query = "p==1".parse().unwrap();
    let drop: [AttributeName; 2] = ["speed".parse().unwrap(), "brandName".parse().unwrap()];
    let keep: [AttributeName; 1] = ["location".parse().unwrap()];
    let purge = PurgeEntities::builder()
        .entity_type(&vehicles)
        .ids(&ids)
        .id_pattern(&pattern)
        .q(&q)
        .geo_q(&geo_q)
        .scope_q(&scope_q)
        .csf(&csf)
        .drop(&drop)
        .keep(&keep)
        .locality(Locality::Local)
        .build();
    let received = PurgeEntitiesParameters::parse(&sent(&purge)).unwrap();
    assert_eq!(received.entity_type.as_ref(), Some(&vehicles));
    assert_eq!(received.ids, ids);
    assert_eq!(received.id_pattern, Some(pattern));
    assert_eq!(received.q, Some(q));
    assert_eq!(received.geo_q, Some(geo_q));
    assert_eq!(received.lone_geoproperty, None);
    assert_eq!(received.scope_q, Some(scope_q));
    assert_eq!(received.csf, Some(csf));
    assert_eq!(received.drop, drop);
    assert_eq!(received.keep, keep);
    assert_eq!(received.locality, Locality::Local);
}

#[test]
fn a_purge_reads_the_parameters_of_its_table_it_does_not_send_and_refuses_others() {
    let received = PurgeEntitiesParameters::parse("type=Vehicle&geoproperty=location&geometryProperty=area").unwrap();
    assert_eq!(received.lone_geoproperty.map(|name| name.to_string()).as_deref(), Some("location"));
    assert_eq!(received.geometry_property.map(|name| name.to_string()).as_deref(), Some("area"));
    assert!(matches!(PurgeEntitiesParameters::parse("attrs=speed").unwrap_err(), ParameterError::Unknown { name } if name == "attrs"));
    assert!(matches!(
        PurgeEntitiesParameters::parse("drop=1speed").unwrap_err(),
        ParameterError::Name {
            parameter: QueryParameter::Drop,
            ..
        }
    ));
}

#[test]
fn a_delete_attribute_reads_back_the_instances_it_removes() {
    let id = vehicle_id();
    let name: AttributeName = "speed".parse().unwrap();
    let gps = DatasetId::from(iri!("urn:ngsi-ld:dataset:gps"));
    let vehicles: TypeSelection = "Vehicle".parse().unwrap();
    for instances in [
        InstanceSelection::Default,
        InstanceSelection::Dataset(&gps),
        InstanceSelection::All,
        InstanceSelection::Selected(None),
        InstanceSelection::Selected(Some(&gps)),
    ] {
        let delete = DeleteAttribute::builder()
            .id(&id)
            .name(&name)
            .instances(instances)
            .entity_type(&vehicles)
            .build();
        let received = DeleteAttributeParameters::parse(&sent(&delete)).unwrap();
        assert_eq!(received.instances(), instances);
        assert_eq!(received.entity_type.as_ref(), Some(&vehicles));
    }
}

#[test]
fn a_delete_attribute_reads_delete_all_as_table_6_7_3_2_1_says() {
    let received = DeleteAttributeParameters::parse("deleteAll=false&datasetId=urn:ngsi-ld:dataset:gps").unwrap();
    assert_eq!(received.delete_all, DeleteAll::Selected);
    assert_eq!(
        received.instances(),
        InstanceSelection::Selected(Some(&DatasetId::from(iri!("urn:ngsi-ld:dataset:gps"))))
    );
    let unflagged = DeleteAttributeParameters::parse("datasetId=urn:ngsi-ld:dataset:gps").unwrap();
    assert_eq!(
        unflagged.instances(),
        InstanceSelection::Dataset(&DatasetId::from(iri!("urn:ngsi-ld:dataset:gps")))
    );
    let every = DeleteAttributeParameters::parse("deleteAll=true&datasetId=urn:ngsi-ld:dataset:gps").unwrap();
    assert_eq!(every.instances(), InstanceSelection::All);
    assert_eq!(
        DeleteAttributeParameters::parse("deleteAll=false").unwrap().instances(),
        InstanceSelection::Selected(None)
    );
    // Clause 4.5.5: `@none` names the default instance wherever a datasetId is given.
    assert_eq!(
        DeleteAttributeParameters::parse("datasetId=%40none").unwrap().instances(),
        InstanceSelection::Dataset(&DatasetId::DefaultInstance)
    );
    assert!(matches!(
        DeleteAttributeParameters::parse("deleteAll=yes").unwrap_err(),
        ParameterError::NotABoolean {
            parameter: QueryParameter::DeleteAll
        }
    ));
    assert!(matches!(
        DeleteAttributeParameters::parse("datasetId=not%20a%20uri").unwrap_err(),
        ParameterError::Uri {
            parameter: QueryParameter::DatasetId,
            ..
        }
    ));
}
