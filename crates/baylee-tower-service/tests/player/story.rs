use super::{
    support::publication,
    transport::{Central, Client, run_response},
};
use baylee_tower::v2::{
    package::Build,
    run::{Command, Phase},
    shop::Delivery,
};
use baylee_tower_api::{Request, Response};
use baylee_tower_store::{Role, Store};
use central_api::Token;
use std::sync::{Arc, atomic::Ordering};
use uuid::Uuid;

pub async fn run(store: Store, central: Arc<Central>, origin: String) {
    let published = publication();
    let cohort = store.publish(&published).await.unwrap();
    let client = Client::new(origin);
    let token = &central.tokens[0];
    let answer = client.send(token, &Request::Cohorts { before: None }).await;
    assert_eq!(answer.0, 200);
    let Response::Cohorts { consent, cohorts } = serde_json::from_slice(&answer.1).unwrap() else {
        panic!("cohorts");
    };
    assert_eq!(cohorts.len(), 1);
    assert_eq!(cohorts[0].id, cohort);
    assert!(store.cohorts(Some(cohort), 8).await.unwrap().is_empty());
    let start = Request::Start {
        request: Uuid::now_v7(),
        cohort,
        starter: 0,
        consent,
    };
    let initial = run_response(client.send(token, &start).await);
    assert_eq!(
        run_response(client.send(token, &start).await),
        initial,
        "no reroll on repeat"
    );
    assert_eq!(
        store
            .runs(central.accounts[0], None, 8)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(
        store
            .runs(central.accounts[0], Some(initial.id), 8)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        store
            .runs(central.accounts[1], None, 8)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(store.runs(central.accounts[0], None, 17).await.is_err());
    let get = Request::Run { run: initial.id };
    assert_eq!(client.send(&central.tokens[1], &get).await.0, 404);
    let buy = Request::Command {
        request: Uuid::now_v7(),
        run: initial.id,
        revision: initial.revision,
        command: Command::Buy {
            offer: initial.shop.offers[0].id,
            delivery: Delivery::Inventory,
        },
    };
    let bought = run_response(client.send(token, &buy).await);
    assert_eq!(bought.revision, initial.revision + 1);
    assert!(bought.economy.gold < initial.economy.gold);
    assert_eq!(run_response(client.send(token, &buy).await), bought);
    assert_eq!(client.send(&central.tokens[1], &buy).await.0, 404);
    let stale = Request::Command {
        request: Uuid::now_v7(),
        run: initial.id,
        revision: initial.revision,
        command: Command::Refresh {},
    };
    assert_eq!(client.send(token, &stale).await.0, 409);
    boundaries(&client, &central, &get).await;
    assert_eq!(
        run_response(client.send(token, &get).await),
        bought,
        "refusals changed no state"
    );
    reserve(&store, &client, token, cohort, &published, &bought).await;
    assert_eq!(client.send(token, &Request::WithdrawEchoes {}).await.0, 200);
}

async fn boundaries(client: &Client, central: &Central, get: &Request) {
    let token = &central.tokens[0];
    let bytes = serde_json::to_vec(get).unwrap();
    let calls = central.calls.load(Ordering::SeqCst);
    assert_eq!(client.raw(None, bytes.clone(), &[]).await.0, 401);
    assert_eq!(
        client
            .raw(
                Some(token),
                bytes.clone(),
                &[("authorization", "Bearer invalid")]
            )
            .await
            .0,
        401
    );
    assert_eq!(
        client
            .raw(
                Some(token),
                bytes.clone(),
                &[("origin", "https://evil.invalid")]
            )
            .await
            .0,
        400
    );
    assert_eq!(
        client
            .raw(Some(token), bytes.clone(), &[("content-encoding", "gzip")])
            .await
            .0,
        400
    );
    assert_eq!(
        client
            .raw(
                Some(token),
                bytes.clone(),
                &[(baylee_tower_api::VERSION_HEADER, "1")]
            )
            .await
            .0,
        426
    );
    assert_eq!(
        client
            .raw(
                Some(token),
                vec![b' '; baylee_tower_api::MAX_REQUEST_BYTES + 1],
                &[]
            )
            .await
            .0,
        413
    );
    assert_eq!(
        client
            .raw(
                Some(token),
                br#"{"operation":"withdraw_echoes","account":"another"}"#.to_vec(),
                &[]
            )
            .await
            .0,
        400
    );
    assert_eq!(
        central.calls.load(Ordering::SeqCst),
        calls,
        "malformed requests reached identity service"
    );
    assert_eq!(client.send(&Token::generate().unwrap(), get).await.0, 401);
    central.wrong_origin.store(true, Ordering::SeqCst);
    assert_eq!(client.send(token, get).await.0, 401);
    central.wrong_origin.store(false, Ordering::SeqCst);
    for (source, expected) in [(401, 401), (403, 403), (503, 503), (307, 503), (429, 429)] {
        central.status.store(source, Ordering::SeqCst);
        assert_eq!(
            client.send(token, get).await.0,
            expected,
            "introspection must not be cached"
        );
    }
    central.status.store(200, Ordering::SeqCst);
}

async fn reserve(
    store: &Store,
    client: &Client,
    token: &Token,
    cohort: Uuid,
    publication: &baylee_tower_store::Publication,
    run: &baylee_tower_api::Run,
) {
    let publisher = Token::generate().unwrap();
    store
        .provision_operator("synthetic publisher", Role::Publisher, publisher.digest())
        .await
        .unwrap();
    let publisher = store
        .authenticate_operator(publisher.digest())
        .await
        .unwrap();
    store
        .publish_house(
            &publisher,
            cohort,
            1,
            1,
            &Build {
                deck: publication.starters[0].deck.clone(),
                active: Vec::new(),
                potions: [0; 2],
            },
        )
        .await
        .unwrap();
    let host = Token::generate().unwrap();
    store
        .provision_operator("synthetic host", Role::Host, host.digest())
        .await
        .unwrap();
    let host = store.authenticate_operator(host.digest()).await.unwrap();
    store
        .advertise(&host, std::slice::from_ref(&publication.cohort.executor))
        .await
        .unwrap();
    let reserve = Request::Reserve {
        request: Uuid::now_v7(),
        run: run.id,
        revision: run.revision,
        host: host.id(),
    };
    let reserved = run_response(client.send(token, &reserve).await);
    assert_eq!(reserved.phase, Phase::Reserved);
    assert_eq!(run_response(client.send(token, &reserve).await), reserved);
    let attempt = Request::Command {
        request: Uuid::now_v7(),
        run: run.id,
        revision: reserved.revision,
        command: Command::Refresh {},
    };
    assert_eq!(client.send(token, &attempt).await.0, 409);
}
