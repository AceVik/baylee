//! Real stdin-only administration against an owned PostgreSQL schema.
#![forbid(unsafe_code)]
#[path = "support/command.rs"]
mod command;
#[path = "admin/process.rs"]
mod process;
#[path = "admin/roles.rs"]
mod roles;
#[path = "../../baylee-tower-store/tests/support/mod.rs"]
mod support;

use baylee_tower::v2::package::Build;
use baylee_tower_store::{CONSENT_REVISION, Error, Start};
use central_api::Token;
use process::{Client, succeeds};
use serde_json::{Value, json};
use uuid::Uuid;

#[tokio::test]
async fn offline_admin_is_idempotent_immutable_role_scoped_and_revocable() {
    let sandbox = support::Sandbox::open().await;
    let store = sandbox.store.clone();
    let client = Client::new(&sandbox.schema);
    let observed = tokio::spawn(async move {
        let publication = support::publication();
        let publish = json!({"action": "publish", "publication": publication});
        let first = client.ok(publish.clone()).await;
        let cohort = id(&first, "cohort");
        assert_eq!(first, json!({"result": "published", "cohort": cohort}));
        assert_eq!(client.ok(publish.clone()).await, first);
        let mut changed = publication.clone();
        changed.starters[0].deck[0].copies += 1;
        client
            .refused(json!({"action": "publish", "publication": changed}))
            .await;
        assert_eq!(store.published(cohort).await.unwrap(), publication);

        let host_token = Token::generate().unwrap();
        let host_command = json!({"action": "provision", "name": "synthetic host",
            "role": "host", "credential": host_token.hex().as_str()});
        let host_reply = client.ok(host_command.clone()).await;
        let host_id = id(&host_reply, "id");
        assert_eq!(client.ok(host_command.clone()).await, host_reply);
        let mut other_role = host_command.clone();
        other_role["role"] = json!("publisher");
        client.refused(other_role).await;
        let mut other_name = host_command.clone();
        other_name["name"] = json!("renamed host");
        client.refused(other_name).await;
        let host = store
            .authenticate_operator(host_token.digest())
            .await
            .unwrap();
        let identities = [publication.cohort.executor.clone()];
        store
            .advertise(&host, &identities, &identities)
            .await
            .unwrap();
        let approval = json!({"action": "approve_host", "host": host_id,
            "origin": "http://127.0.0.1:29873"});
        assert_eq!(client.ok(approval.clone()).await, json!({"result": "done"}));
        client.ok(approval).await;
        client
            .refused(json!({"action": "approve_host", "host": host_id,
            "origin": "http://127.0.0.1:29874"}))
            .await;
        house_commands(&client, cohort, &publication, &host_token).await;

        enrollment(&client, &store, cohort, publish, &first).await;
        assert_eq!(store.published(cohort).await.unwrap(), publication);

        let revoke = json!({"action": "revoke", "operator": host_id});
        client.ok(revoke.clone()).await;
        client.ok(revoke).await;
        client.refused(host_command).await;
        assert!(matches!(
            store.authenticate_operator(host_token.digest()).await,
            Err(Error::NotFound)
        ));
        assert_eq!(
            store.advertise(&host, &identities, &identities).await,
            Err(Error::NotFound)
        );
        for operation in [
            json!({"action": "revoke", "operator": Uuid::now_v7()}),
            json!({"action": "pause_enrollment", "cohort": Uuid::now_v7()}),
        ] {
            client.refused(operation).await;
        }
    })
    .await;
    sandbox.close().await;
    observed.expect("offline operator scenario completed");
}

fn id(reply: &Value, field: &str) -> Uuid {
    let id: Uuid = reply[field].as_str().unwrap().parse().unwrap();
    assert_eq!(id.get_version_num(), 7);
    id
}

async fn enrollment(
    client: &Client,
    store: &baylee_tower_store::Store,
    cohort: Uuid,
    publish: Value,
    first: &Value,
) {
    let account = Uuid::now_v7();
    let request = Uuid::now_v7();
    let choice = Start {
        cohort,
        starter: 0,
        consent: CONSENT_REVISION,
    };
    let run = store
        .start(account, request, choice, [1; 32])
        .await
        .unwrap();
    let pause = json!({"action": "pause_enrollment", "cohort": cohort});
    client.ok(pause.clone()).await;
    client.ok(pause).await;
    assert_eq!(
        store
            .start(account, request, choice, [1; 32])
            .await
            .unwrap(),
        run
    );
    assert_eq!(
        store
            .start(Uuid::now_v7(), Uuid::now_v7(), choice, [2; 32])
            .await,
        Err(Error::NotFound)
    );
    // Republishing an exact payload cannot reopen enrollment.
    assert_eq!(&client.ok(publish).await, first);
    assert_eq!(
        store
            .start(Uuid::now_v7(), Uuid::now_v7(), choice, [2; 32])
            .await,
        Err(Error::NotFound)
    );
}

async fn house_commands(
    client: &Client,
    cohort: Uuid,
    publication: &baylee_tower_store::Publication,
    host: &Token,
) {
    let publisher = Token::generate().unwrap();
    let reply = client
        .ok(json!({"action": "provision", "name": "synthetic publisher",
        "role": "publisher", "credential": publisher.hex().as_str()}))
        .await;
    let build = Build {
        formation: None,
        deck: publication.starters[0].deck.clone(),
        active: Vec::new(),
        potions: [0; 2],
    };
    let publish = json!({"action": "publish_house", "publisher": publisher.hex().as_str(),
        "cohort": cohort, "stage": 1, "archetype": 1, "build": build});
    let mut wrong = publish.clone();
    wrong["publisher"] = json!(host.hex().as_str());
    client.refused(wrong).await;
    client.ok(publish.clone()).await;
    client.ok(publish.clone()).await;
    let mut changed = publish.clone();
    changed["build"]["potions"] = json!([1, 0]);
    client.refused(changed).await;
    client
        .ok(json!({"action": "revoke", "operator": id(&reply, "id")}))
        .await;
    client.refused(publish).await;
}

/// A run whose duel is reserved on a host that never starts it.
async fn stuck_run(
    store: &baylee_tower_store::Store,
) -> (
    Uuid,
    baylee_tower_store::Snapshot,
    baylee_tower_store::Snapshot,
) {
    use baylee_tower::v2::package::Seeds;
    use baylee_tower_store::{Entropy, Reserve, Role};
    let publication = support::publication();
    let cohort = store.publish(&publication).await.unwrap();
    let principal = |role: Role, secret: u8| {
        let store = store.clone();
        async move {
            store
                .provision_operator("synthetic supervision actor", role, [secret; 32])
                .await
                .unwrap();
            store.authenticate_operator([secret; 32]).await.unwrap()
        }
    };
    let host = principal(Role::Host, 2).await;
    let publisher = principal(Role::Publisher, 3).await;
    let identities = [publication.cohort.executor.clone()];
    store
        .advertise(&host, &identities, &identities)
        .await
        .unwrap();
    store
        .publish_house(
            &publisher,
            cohort,
            1,
            1,
            &Build {
                formation: None,
                deck: publication.starters[0].deck.clone(),
                active: Vec::new(),
                potions: [0; 2],
            },
        )
        .await
        .unwrap();
    let account = Uuid::now_v7();
    let run = store
        .start(
            account,
            Uuid::now_v7(),
            Start {
                cohort,
                starter: 0,
                consent: CONSENT_REVISION,
            },
            [1; 32],
        )
        .await
        .unwrap();
    let reserved = store
        .reserve(
            account,
            Uuid::now_v7(),
            Reserve {
                run: run.id,
                revision: run.state.revision,
                host: host.id(),
            },
            Entropy {
                opponent: [4; 32],
                game: Seeds {
                    shuffle: 4,
                    policy: 5,
                },
            },
            4096,
        )
        .await
        .unwrap();
    (account, run, reserved)
}

#[tokio::test]
async fn a_supervisor_credential_quarantines_and_retires_a_stuck_run_offline() {
    use baylee_tower::v2::run::Phase;
    let sandbox = support::Sandbox::open().await;
    let store = sandbox.store.clone();
    let client = Client::new(&sandbox.schema);
    let observed = tokio::spawn(async move {
        let (account, run, reserved) = stuck_run(&store).await;
        let supervisor = Token::generate().unwrap();
        client
            .ok(
                json!({"action": "provision", "name": "synthetic supervisor",
                "role": "supervisor", "credential": supervisor.hex().as_str()}),
            )
            .await;
        let supervise = |credential: &Token, revision: u64, recovery: Value| {
            json!({"action": "supervise", "supervisor": credential.hex().as_str(),
                "run": run.id, "revision": revision, "recovery": recovery})
        };
        let quarantine = json!({"kind": "quarantine"});
        // Another role's credential, or a stale revision, changes nothing.
        let wrong = Token::generate().unwrap();
        client
            .refused(supervise(
                &wrong,
                reserved.state.revision,
                quarantine.clone(),
            ))
            .await;
        client
            .refused(supervise(
                &supervisor,
                reserved.state.revision + 1,
                quarantine.clone(),
            ))
            .await;
        let reply = client
            .ok(supervise(&supervisor, reserved.state.revision, quarantine))
            .await;
        let held = store.get(account, run.id).await.unwrap();
        assert_eq!(held.state.phase, Phase::Quarantined);
        assert_eq!(
            reply,
            json!({"result": "supervised", "revision": held.state.revision})
        );
        client
            .ok(supervise(
                &supervisor,
                held.state.revision,
                json!({"kind": "interrupt"}),
            ))
            .await;
        let retired = store.get(account, run.id).await.unwrap();
        assert_eq!(retired.state.phase, Phase::Interrupted);
        assert_eq!(retired.state.economy, run.state.economy);
    })
    .await;
    sandbox.close().await;
    observed.expect("offline supervision scenario completed");
}

#[tokio::test]
async fn check_admin_is_offline_and_diagnostics_never_echo_input() {
    let client = Client {
        configuration: json!({"database_url": "postgres://qa@127.0.0.1:1/synthetic",
        "schema": "tower_test", "development": true}),
    };
    let operation = json!({"action": "provision", "name": "PRIVATE_OPERATOR_CANARY",
        "role": "host", "credential": "11".repeat(32)});
    let output = client.call("--check-admin", operation.clone()).await;
    succeeds(&output);
    assert!(output.stdout.is_empty());
    let mut unknown = operation;
    unknown["unknown"] = json!("PRIVATE_INPUT_CANARY");
    let output = client.call("--check-admin", unknown).await;
    process::refused(&output);
}
