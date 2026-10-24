//! Tenants (clauses 4.14 and 5.5.10) and the `NGSILD-Tenant` header (clause 6.3.14).

use crate::broker::{client, create, entity_id, parked_vehicle, unique};
use ngsi_ld::{
    client::{ClientError, ReqwestError},
    http::Retrieved,
    model::{Entity, ErrorType, Output, Tenant},
};
use pretty_assertions::assert_eq;

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn an_entity_created_in_a_tenant_is_found_there_and_not_in_the_default_tenant() {
    let client = client();
    let tenant: Tenant = unique("tenant").parse().unwrap();
    let id = entity_id("tenant");
    let vehicle = parked_vehicle(&id);
    let created = client.entities().create(&vehicle).tenant(&tenant).send().await.unwrap();
    assert!(created.is_created(), "{created:?}");
    let in_default = client.entities().retrieve(&id).send().await.map(Retrieved::into_body);
    let in_tenant = client.entities().retrieve(&id).tenant(&tenant).send().await.map(Retrieved::into_body);
    client.entities().delete(&id).tenant(&tenant).send().await.unwrap();
    let error = in_default.unwrap_err();
    assert_eq!(error.error_type(), Some(&ErrorType::ResourceNotFound), "{error:?}");
    assert_eq!(in_tenant.unwrap().into_input().unwrap(), vehicle);
}

/// Creates an entity in the default tenant and retrieves it from a tenant that does not exist.
async fn retrieve_from_an_absent_tenant(name: &str) -> Result<Entity<Output>, ClientError<ReqwestError>> {
    let client = client();
    let id = entity_id(name);
    let vehicle = parked_vehicle(&id);
    create(&client, &vehicle).await;
    let tenant: Tenant = unique("absent").parse().unwrap();
    let outcome = client.entities().retrieve(&id).tenant(&tenant).send().await.map(Retrieved::into_body);
    client.entities().delete(&id).send().await.unwrap();
    outcome
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn retrieving_from_a_tenant_that_does_not_exist_does_not_return_the_entity() {
    // Clause 5.5.10: an operation on a tenant shall only be applied to information of that tenant, so an entity of
    // the default tenant is not found through another one, and the broker answers with an error.
    let error = retrieve_from_an_absent_tenant("absentTenant").await.unwrap_err();
    assert!(matches!(error, ClientError::Response(_)), "{error:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn retrieving_from_a_tenant_that_does_not_exist_should_be_nonexistent_tenant() {
    // Known deviation from a recommendation, clause 5.5.10: an operation other than a creation that targets a tenant
    // that does not exist "should" raise NonexistentTenant (404 in Table 6.3.2). It is a "should", not a "shall", so
    // a broker that answers otherwise still conforms; the test records the recommendation. Observed: the broker
    // answers 404 with its own problem type `https://uri.etsi.org/ngsi-ld/errors/TenantNotFound`.
    let error = retrieve_from_an_absent_tenant("unknownTenant").await.unwrap_err();
    assert_eq!(error.error_type(), Some(&ErrorType::NonexistentTenant), "{error:?}");
}
