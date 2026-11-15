use super::{Runtime, Served, common, fresh, native, recovery};
use baylee_account::TowerHttp;
use baylee_protocol::v1;
use baylee_tower::v2::{
    DUEL_INCOME,
    run::{Outcome, Phase},
};
use baylee_tower_api::{Request, Response, Run};
use std::time::{Duration, Instant};
use uuid::Uuid;

fn owned(client: &TowerHttp, request: &Request) -> Run {
    let Response::Run { run } = client.exchange(request).unwrap() else {
        panic!("owned Tower run");
    };
    *run
}

pub(super) fn run(runtime: &Runtime, served: &Served) {
    let fixture = served.tower.as_ref().unwrap();
    let account = player(runtime, served, "tower_player");
    let admission = account.admit(&served.origin).unwrap();
    let client = account.tower().unwrap();
    assert_eq!(client.origin(), fixture.origin);
    let mut agent = runtime.block_on(native::agent(&served.gateway, fixture));
    let (status, availability) = common::http(
        served.gateway.port,
        "GET",
        "/tower",
        Some(&admission.token),
        "",
    );
    assert_eq!(status, 200, "{availability}");
    let availability: serde_json::Value = serde_json::from_str(&availability).unwrap();
    assert_eq!(
        availability["engines"][0],
        serde_json::to_value(&fixture.identity).unwrap()
    );
    let host = Uuid::parse_str(availability["host"].as_str().unwrap()).unwrap();
    let initial = owned(
        &client,
        &Request::Start {
            request: Uuid::now_v7(),
            cohort: fixture.cohort,
            starter: 0,
            consent: baylee_tower_store::CONSENT_REVISION,
        },
    );
    let reserve = Request::Reserve {
        request: Uuid::now_v7(),
        run: initial.id,
        revision: initial.revision,
        host,
    };
    let reserved = owned(&client, &reserve);
    assert_eq!(reserved.phase, Phase::Reserved);
    assert_eq!(owned(&client, &reserve), reserved);
    let path = format!("/tower/{}/play", initial.id);
    refusals(runtime, served, &admission.token, &path, &client, &reserved);
    let grant = start(served, &admission.token, &path);
    let again = start(served, &admission.token, &path);
    assert_eq!(again["game_id"], grant["game_id"]);
    assert_ne!(again["seat_token"], grant["seat_token"]);
    let pending = recovery::crash(runtime, served, initial.id, &again, &mut agent);
    let restored = start(served, &admission.token, &path);
    assert_ne!(
        restored["game_id"], grant["game_id"],
        "a fresh execution has a separate transport identity"
    );
    runtime.block_on(async {
        let v1::envelope::Msg::StartEngine(start) = native::next(&mut agent).await else {
            panic!("exact Tower start");
        };
        assert_eq!(start.game_id, restored["game_id"].as_str().unwrap());
        recovery::stale_status(&mut agent, grant["game_id"].as_str().unwrap()).await;
        let mut engine = native::Engine::start(&start, fixture);
        native::concede(&served.gateway, &restored, &pending).await;
        engine.finished().await;
        agent.close(None).await.unwrap();
    });
    settlement(served, &client, &initial, &restored, &admission.token);
    println!(
        "Real Central identity → gateway → exact native Tower → durable records → independent automatic verification passed; one defeat and one gold payment."
    );
}

fn player(runtime: &Runtime, served: &Served, name: &str) -> baylee_account::Account {
    fresh::register(runtime, &served.service, &served.endpoint, name, br#"{"version":1,"gateways":[],"preferences":{},"legacy":null,"chat":{"devices":[],"installations":[]}}"#)
}

fn refusals(
    runtime: &Runtime,
    served: &Served,
    token: &str,
    path: &str,
    client: &TowerHttp,
    reserved: &Run,
) {
    let (status, _) = common::http(
        served.gateway.port,
        "POST",
        path,
        Some(token),
        r#"{"account":"untrusted"}"#,
    );
    assert!(matches!(status, 400 | 422));
    assert_eq!(&owned(client, &Request::Run { run: reserved.id }), reserved);
    let foreign = player(runtime, served, "tower_foreign");
    let foreign = foreign.admit(&served.origin).unwrap();
    assert_eq!(
        common::http(
            served.gateway.port,
            "POST",
            path,
            Some(&foreign.token),
            "{}"
        )
        .0,
        403
    );
    assert_eq!(
        &owned(client, &Request::Run { run: reserved.id }),
        reserved,
        "a different verified player cannot consume this reservation"
    );
}

fn start(served: &Served, token: &str, path: &str) -> serde_json::Value {
    let (status, grant) = common::http(served.gateway.port, "POST", path, Some(token), "{}");
    assert_eq!(status, 200, "{grant}");
    serde_json::from_str(&grant).unwrap()
}

fn settlement(
    served: &Served,
    client: &TowerHttp,
    initial: &Run,
    grant: &serde_json::Value,
    token: &str,
) {
    let deadline = Instant::now() + Duration::from_secs(20);
    let settled = loop {
        let run = owned(client, &Request::Run { run: initial.id });
        if run.history.len() == 1 {
            break run;
        }
        assert!(
            Instant::now() < deadline,
            "independent automatic adjudication finishes"
        );
        std::thread::sleep(Duration::from_millis(100));
    };
    assert_eq!(settled.phase, Phase::Shop);
    assert_eq!(settled.history[0].outcome, Outcome::Defeat);
    assert_eq!(settled.economy.gold, initial.economy.gold + DUEL_INCOME);
    assert_eq!(settled.history[0].used, [0; 2]);
    let rematch = format!(
        "/lobby/games/{}/rematch",
        grant["game_id"].as_str().unwrap()
    );
    assert_eq!(
        common::http(served.gateway.port, "POST", &rematch, Some(token), "{}").0,
        409
    );
    assert_eq!(
        owned(client, &Request::Run { run: initial.id }),
        settled,
        "retries and rematch refusal cannot award a second income"
    );
}
