# Live testing

Most of the crate's tests run without a network. The binding tests in `tests/binding.rs` check the exact request each operation builds and how each response is decoded. The golden tests read and write the Annex C examples.

The live tests go further. They send every client operation to a real broker and check that it answers the way ETSI GS CIM 009 v1.9.1 says it should. They test the specification, not whatever the broker happens to do.

Every live test is `#[ignore]`d and needs `NGSI_LD_TEST_BROKER_URL`, so a plain `cargo test` never touches the network. No workflow runs them either. You run them by hand, against whichever broker you like.

The examples in `examples/` can talk to the same broker, but they only show how to use the client. Nothing checks their output against the specification, and no workflow runs them.

## Setup

Start a broker and point these variables at its API root, the URL that `/entities` is appended to.

| Variable | Used by | Example |
| --- | --- | --- |
| `NGSI_LD_BROKER_URL` | the examples in `examples/` | `http://localhost:9090/ngsi-ld/v1/` |
| `NGSI_LD_TEST_BROKER_URL` | the live tests in `tests/live.rs` | `http://localhost:9090/ngsi-ld/v1/` |
| `NGSI_LD_TEST_NOTIFICATION_HOST` | the notification tests in `tests/live/subscriptions.rs` and `tests/live/registrations.rs` | `host.docker.internal` |

## Running the examples

```sh
NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo run --all-features --example query_pagination
```

These examples need a broker: `create_entity`, `representations`, `query_pagination`, `merge_fragment`, `dataset_instances`, `replace_entity`, `append_update_attributes`, `delete_purge`, `unchecked_rejections`, `tenants`, `batch_operations`, `subscriptions`, `temporal`, `registrations`, `discovery`, `entity_maps`, `snapshots` and `geo_scope_query`. Each one explains in its header what it shows, and prints what it does as it runs. They use fixed ids that look like the specification's. Each deletes whatever an earlier run left behind before it starts and cleans up after itself, so you can run it again against the same broker.

These need no broker:

- `documents`: the data model
- `query_languages`: the query languages
- `notifications`: receiving notifications
- `linked_data`: the `linked-data` feature
- `conversions`: the conversion features
- `context_source`: acting as a context source
- `sans_io`: the HTTP layer without a client

## Running the live tests

```sh
NGSI_LD_TEST_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo test --all-features --test live -- --ignored
```

Each test uses its own ids, entity types and tenants, unique to that run. So the tests can run in parallel, and you can rerun them without cleaning out the broker. They send no `@context`, which means every term expands under the core context's default vocabulary. A broker that can't fetch remote contexts can still run them.

### Notification tests

The notification tests start a receiver inside the test process. It listens on every interface, on a free port. The tests then subscribe with an endpoint on the host in `NGSI_LD_TEST_NOTIFICATION_HOST`, which has to be the address the broker uses to reach the machine running the tests. If the broker runs in a container on the same machine, that's `host.docker.internal`. Docker Desktop defines that name, and on Docker Engine for Linux the compose stack below maps it.

```sh
NGSI_LD_TEST_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ NGSI_LD_TEST_NOTIFICATION_HOST=host.docker.internal cargo test --all-features --test live -- --ignored
```

### Registration tests

The tests in `tests/live/registrations.rs` change the broker's registry of context sources, and a broker checks that registry for every request it might distribute (clause 5.12). Run them last, one at a time, so that no other test sends a request while the registry is changing:

```sh
NGSI_LD_TEST_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ NGSI_LD_TEST_NOTIFICATION_HOST=host.docker.internal cargo test --all-features --test live -- --ignored --skip registrations::
NGSI_LD_TEST_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ NGSI_LD_TEST_NOTIFICATION_HOST=host.docker.internal cargo test --all-features --test live -- --ignored --test-threads=1 registrations::
```

## Recording a deviation

When a broker's answer doesn't match the specification, the test keeps asserting the specification. Give it a comment that starts with `Known deviation`, cites the clause and says plainly what the broker did, then add the test to the table below.

Sometimes the clause only says a broker *should* do something. Record that the same way, but have the comment, the test name and the table row call it a recommendation, so it never reads as non-conformance.

Never change the library to accept a deviation, and never weaken an assertion to let a broker pass. When a deviation goes away, the test starts passing again, and its comment and table row come out.

## The compose stack

`tests/live/compose.yaml` runs Scorpio, the NGSI-LD broker of the FIWARE ecosystem, in its all-in-one form (`scorpiobroker/all-in-one-runner:java-6.0.2`). It listens on port 9090 and stores its data in PostgreSQL with PostGIS (`postgis/postgis:17-3.5`). Both images are amd64 only, so on any other architecture they run under emulation.

Start it and wait until both services report healthy. When you're done, stop it and drop its data:

```sh
docker compose -f tests/live/compose.yaml up -d --wait
docker compose -f tests/live/compose.yaml down -v
```

The API root is `http://localhost:9090/ngsi-ld/v1/`. Scorpio creates a tenant the first time an entity is created in it, and a request without an `NGSILD-Tenant` header goes to the default tenant.

### Observed deviations

Seen with `scorpiobroker/all-in-one-runner:java-6.0.2`. Each row is a live test that asserts the specification and fails against this broker.

| Test | Clause | Specification | Observed |
| --- | --- | --- | --- |
| `entities::the_annex_vehicle_is_created_and_retrieved_in_normalized_form_as_sent` | 4.5.21, Table 5.2.36-1 | A ListProperty's `valueType` (`"Integer"`) comes back as sent. | `"valueType": null`. That isn't an NGSI-LD value, so the entity can't be read. |
| `entities::the_annex_vehicle_is_retrieved_in_concise_form_as_sent` | 4.5.21, Table 5.2.36-1 | Same, in concise form. | Same. |
| `entities::every_attribute_kind_is_retrieved_in_simplified_form_as_clause_4_5_4_writes_it` | 4.5.4, 4.5.18, 4.5.20, 4.5.22 | A LanguageProperty is `{"languageMap": {...}}`, a VocabProperty is `{"vocab": ...}`, and a ListRelationship is the array of its object ids. | The bare language map, the bare term, and an array of `{"object": ...}` objects. |
| `entities::merge_sets_an_attribute_and_deletes_one_written_as_the_bare_null` | 4.5.0, 4.5.3.3, 5.4 | The bare NGSI-LD Null, `"isParked": "urn:ngsi-ld:null"`, deletes the attribute. | 400 BadRequestData, "The key isParked is an invalid entry." A Null written in the shape of the attribute's kind is accepted. |
| `query::query_through_post_follows_the_next_links_to_every_match` | 6.3.10, 6.23.3.1 | The `next` link leads to the next page of the same query. | The link is `/ngsi-ld/v1/entities?count=true&limit=2&offset=2`. It drops the type, `q` and `pick` from the query body, and following it gets 400 InvalidRequest. |
| `query::query_with_limit_zero_and_count_returns_the_total_and_no_entity` | 6.3.13, 6.4.3.2 | 200 with `NGSILD-Results-Count` and an empty JSON array. | 200 with `NGSILD-Results-Count` and an empty body, which isn't JSON. |
| `attributes::append_attributes_without_overwrite_keeps_existing_ones_and_reports_them_not_updated` | 6.6.3.1, 5.2.18 | The existing attribute is kept and listed in the `notUpdated` of a 207 answer. | The attribute is kept, but the answer is 204. |
| `attributes::partial_attribute_update_changes_the_members_it_names_and_keeps_the_others` | 6.7.3.1 | Only the members the fragment names change. | The whole attribute is replaced, so the `unitCode` the fragment leaves out is lost. |
| `attributes::partial_attribute_update_of_a_missing_attribute_is_resource_not_found` | 6.7.3.1 | 404 ResourceNotFound. | 204. |
| `attributes::replace_attribute_replaces_the_whole_instance` | 6.7.3.3 | 204, and the attribute is replaced. | 400 BadRequestData, "resolved size 1", for every Replace Attribute. |
| `attributes::replace_attribute_of_a_missing_attribute_is_resource_not_found` | 6.7.3.3 | 404 ResourceNotFound. | 400 BadRequestData, "resolved size 1". |
| `attributes::delete_attribute_with_a_dataset_id_removes_only_that_instance` | 6.7.3.2 | The instance with that `datasetId` is deleted. | 404 ResourceNotFound, "Attribute .../speed with datasetId urn:ngsi-ld:dataset:gps ... was not found.", for an instance Retrieve Entity returns. |
| `removal::purge_by_type_deletes_every_entity_of_the_type` | 6.4.3.3 | `DELETE /entities` purges the matching entities. | 405 with the problem type `https://uri.etsi.org/ngsi-ld/errors/MethodNotAllowed`, which Table 5.5.2-1 doesn't define. |
| `removal::purge_with_drop_removes_only_the_dropped_attributes` | 6.4.3.3 | The `drop` attributes are removed from every matching entity. | 405, as above. |
| `removal::purge_with_keep_removes_every_other_attribute` | 6.4.3.3 | Every attribute except the `keep` ones is removed. | 405, as above. |
| `geo_scope::a_scope_query_with_alternatives_matches_only_the_queried_type` | 5.7.2.4, 4.19 | A query for one type with the scope query `/root/Barcelona\|/root/Madrid/Sights` returns only entities of that type in one of those scopes. | An entity of another type in `/root/Madrid/Sights` comes back too, through both GET and POST. The broker acts as if the type applies only to the first alternative. |
| `geo_scope::purge_by_scope_query_deletes_only_the_entities_in_scope` | 6.4.3.3 | The entities of that type in the scope are purged. | 405, as for every purge. |
| `tenants::retrieving_from_a_tenant_that_does_not_exist_should_be_nonexistent_tenant` | 5.5.10 (should), Table 6.3.2 | Recommended, not required: the error type NonexistentTenant, with 404. | 404 with the problem type `https://uri.etsi.org/ngsi-ld/errors/TenantNotFound`, which Table 5.5.2-1 doesn't define. |
| `subscriptions::query_subscriptions_follows_the_next_links_to_every_subscription` | 5.5.9, 6.3.10 | A Query Subscriptions page links to the next page while more subscriptions remain. | A `limit=1` page never has a `next` link, however many subscriptions follow, so only the first page can be reached. |
| `subscriptions::query_subscriptions_with_limit_zero_and_count_returns_the_total_and_no_subscription` | 6.3.13 | 200 with `NGSILD-Results-Count` and an empty JSON array. | 200 with an empty body, which isn't JSON, and no `NGSILD-Results-Count`. |
| `subscriptions::the_notified_attributes_are_retrieved_as_an_array` | Table 5.2.14.1-1 | `notification.attributes` is an array of names. | The single name sent comes back as a bare string (`"attributes": "speed"`), which the reader rejects. |
| `subscriptions::a_simplified_notification_reports_the_new_speed` | 5.2.14, 4.5.4 | A subscription whose `format` is `simplified` is notified with simplified entities. | The entities arrive in normalized form (`"speed": {"type": "Property", "value": 55}`). |
| `temporal::retrieve_reads_the_simplified_temporal_representation` | Table 6.3.12-1 | `format=temporalValues` asks for the simplified temporal representation. | 500 InternalError. The broker serves that representation only for `options=temporalValues`, which the table deprecates. |
| `temporal::retrieve_reads_the_aggregated_temporal_representation` | Table 6.3.12-1 | `format=aggregatedValues` asks for the aggregated temporal representation. | 500 InternalError. The broker serves that representation only for `options=aggregatedValues`, which the table deprecates. |
| `temporal::retrieve_reads_the_concise_temporal_representation` | 6.3.7 | `format=concise` asks for the concise representation. | 500 InternalError on `temporal/entities/{entityId}`. |
| `temporal::several_instances_of_an_attribute_are_added_at_once` | 5.6.12.4 | Every instance in the fragment is added ("the Attribute instance(s) shall be added"). | 400 BadRequestData, "Duplicated datasetId or multiple entries with no datasetId found", for two instances of the default dataset. One instance is added. |
| `registrations::a_registration_is_registered_retrieved_updated_and_deleted` | Table 5.2.9-1, RFC 7230 section 5.7.1 | `contextSourceAlias` is a pseudonym, which is an HTTP token. | When a registration sends no alias, the broker fills in the endpoint URL (`http://my.csource.org:1026/`). A URL isn't a token, so the registration can't be read. |
| `registrations::query_registrations_by_type_finds_the_registration` | Table 5.2.9-1, RFC 7230 section 5.7.1 | Same. | Same, for the registrations on a query page. |
| `registrations::a_registration_without_an_id_is_named_by_the_location` | Table 5.2.9-1, RFC 7230 section 5.7.1 | Same. | Same. |
| `registrations::a_registration_subscription_is_created_retrieved_updated_and_deleted` | Tables 5.2.12-1, 5.2.14-2 | A retrieved registration subscription has the members of Table 5.2.12-1, with the notification statistics inside `notification`. | `lastFailure`, `lastNotification`, `timesFailed` and `timesSent` sit at the top level, and the timestamps are written as `"$1"`, which the reader rejects. |
| `registrations::a_registration_subscription_without_an_id_is_named_by_the_location` | 5.11.2.4 | A registration subscription without an `id` is given one and created. | 409 AlreadyExists, "Subscription with id null exists". |
| `registrations::a_registration_subscription_is_notified_of_a_newly_matching_registration` | 5.11.7, 6.3.9 | A new registration that matches the subscription triggers a notification with `triggerReason` `newlyMatching`. | No notification arrives, even though the broker can reach the receiver. |
| `discovery::a_single_entity_type_is_listed_in_detail_as_an_array` | Table 6.25.3.1-2, Table 5.2.25-1 | The detailed entity type list is an array of `EntityType` objects, each with an array of `attributeNames`, however many elements there are. | A single entity type comes back as a bare object, and a single attribute name as a bare string (`"attributeNames": "speed"`). An empty entity type list names its member `ngsi-ld:typeList`. |
| `discovery::the_metadata_of_a_hosted_context_is_listed_and_served` | 5.13.3.5, 5.13.2.4 | A stored context's metadata has the members `URL`, `localId`, `kind`, `createdAt`, `expiresAt`, `lastUsage`, `numberOfHits` and `extraInfo`, and its `localId` is a URI. | The metadata also carries the context itself as `body`, which the reader rejects. One of the hosted contexts has the `localId` `)$%^&`, which isn't a URI. |
| `entity_maps::a_query_names_a_map_that_can_be_retrieved_updated_and_deleted` | 6.32.3.1, Table 6.2-1, Table 5.2.39-2 | The map a query names can be retrieved, updated and deleted at `/entityMaps/{entityMapId}`, and it has a `linkedMaps` member. | 405 MethodNotAllowed at `/entityMaps/{entityMapId}`. The broker serves the map at `/entityMap/{entityMapId}` instead, without `linkedMaps`. The query names the map in `NGSILD-EntityMap` with a bare id instead of a location, and the id can still be read from it. |
| `entity_maps::a_map_created_for_a_query_by_get_and_by_post_pages_the_entities` | 6.34.3.1, 6.34.3.2 | `GET` and `POST /entityMaps` create a map and answer 201 with it. | 405 MethodNotAllowed to both. |
| `entity_maps::a_map_created_for_a_temporal_query_holds_the_recorded_entity` | 6.35.3.1, 6.35.3.2 | `GET` and `POST /temporal/entityMaps` create a map and answer 201 with it. | 405 MethodNotAllowed. |
| `entity_maps::the_broker_names_itself` | 5.15.1.4, 5.15.1.5, Table 5.2.40-1 | A Context Source Identity, or 501 if the broker can't supply one. | 200 with the body `http://localhost:9090/)$%^&`, which isn't JSON. |
| `snapshots::a_snapshot_freezes_the_entities_it_queries` | 6.36.3.1, 5.16.1 | `POST /snapshots` creates the snapshot and answers 201 with its location. | 405 MethodNotAllowed. The broker doesn't implement the Snapshot API. Clause 4.3.5 makes that API optional, but Table 6.2-1 defines the resource. |
| `snapshots::a_snapshot_is_cloned_updated_and_deleted` | 6.36.3.1, 6.37.3, 6.38.3.1 | A snapshot is created, cloned, updated and deleted. | 405 MethodNotAllowed to `POST /snapshots`, as above. |
| `snapshots::snapshots_of_a_low_priority_are_purged` | 6.36.3.1, 6.36.3.2 | `DELETE /snapshots?q=...` purges the snapshots it selects. | 405 MethodNotAllowed to `POST /snapshots`, as above. |
| `request_options::a_version_preference_is_confirmed_by_preference_applied` | 6.3.6, 4.3.6.8 | With `Prefer: ngsi-ld=<version>`, the answer has `Preference-Applied: ngsi-ld=<conformant-version>`. | No `Preference-Applied` header. |

If an example calls an operation this broker rejects, such as Create Snapshot, the example stops with that error.

Creating or deleting a context source registration while other requests are in flight can make those requests fail with 500 InternalError ("Cannot invoke "java.util.List.forEach(java.util.function.Consumer)" because the return value of "com.google.common.collect.Table.get(Object, Object)" is null"), whatever entity type they're about. It has shown up on Batch Entity Upsert and Delete Entity. Clause 5.12 makes the registry part of every request a broker may distribute, but no clause lets a registry change break an unrelated request. Which request fails varies from run to run, so no test records it. The registry tests run on their own instead, as [Registration tests](#registration-tests) describes.
