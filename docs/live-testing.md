# Live Testing

The live tests send every client operation to a real broker and check that it answers the way ETSI GS CIM 009 v1.9.1 says it should. They test the specification, not whatever the broker happens to do. Each one is `#[ignore]`d, so a plain `cargo test` never touches the network, and no workflow runs them.

## Setup

Start a broker and point these variables at its API root, the URL that `/entities` is appended to.

| Variable | Used by | Example |
| --- | --- | --- |
| `NGSI_LD_BROKER_URL` | the examples | `http://localhost:9090/ngsi-ld/v1/` |
| `NGSI_LD_TEST_BROKER_URL` | the live tests | `http://localhost:9090/ngsi-ld/v1/` |
| `NGSI_LD_TEST_NOTIFICATION_HOST` | the notification tests | `host.docker.internal` |

## Running the Examples

```sh
NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo run --all-features --example query_pagination
```

Most examples talk to the broker. They clean up after themselves, so you can run them again. `documents`, `query_languages`, `notifications`, `linked_data`, `conversions`, `context_source` and `sans_io` need no broker.

## Running the Live Tests

```sh
NGSI_LD_TEST_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ NGSI_LD_TEST_NOTIFICATION_HOST=host.docker.internal cargo test --all-features --test live -- --ignored --skip registrations::
NGSI_LD_TEST_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ NGSI_LD_TEST_NOTIFICATION_HOST=host.docker.internal cargo test --all-features --test live -- --ignored --test-threads=1 registrations::
```

Each test uses ids, types and tenants of its own, so the tests run in parallel and need no cleanup between runs. The registration tests change the registry a broker consults for every request it may distribute (clause 5.12), so they run last and one at a time.

The notification tests receive notifications in the test process. `NGSI_LD_TEST_NOTIFICATION_HOST` is the address the broker uses to reach the machine running the tests. For a broker in a local container, that's `host.docker.internal`.

## Recording a Deviation

When a broker's answer doesn't match the specification, the test keeps asserting the specification. Give it a comment that starts with `Known deviation`, cites the clause and says what the broker did. A clause that only says *should* is recorded the same way, as a recommendation.

Never change the library or weaken an assertion to let a broker pass. When a deviation goes away, the test passes again and its comment comes out.

## The Compose Stack

`tests/live/compose.yaml` runs Scorpio with PostgreSQL on port 9090. Its images are amd64 only, so other architectures run them under emulation.

```sh
docker compose -f tests/live/compose.yaml up -d --wait
docker compose -f tests/live/compose.yaml down -v
```

Some live tests fail against it. Each of those tests carries its `Known deviation` comment.
