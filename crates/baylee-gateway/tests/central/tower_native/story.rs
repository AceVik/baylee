use super::{Runtime, Served, common, fresh, native, recovery};
use baylee_account::TowerHttp;
use baylee_protocol::v1;
use baylee_tower::v2::{
    DUEL_INCOME,
    run::{Command, Outcome, Phase},
};
use baylee_tower_api::{Request, Response, Run};
use std::time::{Duration, Instant};
use uuid::Uuid;

#[track_caller]
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
    let Response::Cohort { cohort: draft } = client
        .exchange(&Request::Cohort {
            cohort: fixture.cohort,
        })
        .unwrap()
    else {
        panic!("owner-bound starting choices");
    };
    draft.validate().unwrap();
    assert_eq!(draft.offered.len(), 3);
    let initial = owned(
        &client,
        &Request::Start {
            request: Uuid::now_v7(),
            cohort: fixture.cohort,
            starter: draft.offered[0],
            consent: baylee_tower_store::CONSENT_REVISION,
        },
    );
    let initial = shop(&client, &initial);
    let discovered = discover(&client, initial.id, &served.origin);
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
    assert_eq!(host, discovered);
    let reserve = Request::Reserve {
        request: Uuid::now_v7(),
        run: initial.id,
        revision: initial.revision,
        host,
    };
    let reserved = owned(&client, &reserve);
    assert_eq!(reserved.phase, Phase::Reserved);
    assert_eq!(owned(&client, &reserve), reserved);
    location(&client, initial.id, &served.origin, None);
    let path = format!("/tower/{}/play", initial.id);
    refusals(runtime, served, &admission.token, &path, &client, &reserved);
    let grant = start(served, &admission.token, &path);
    busy(served, &client, initial.id, &admission.token);
    let again = sdk_start(&account, &client, initial.id, &served.origin);
    assert_eq!(again["game_id"], grant["game_id"]);
    assert_ne!(again["seat_token"], grant["seat_token"]);
    let again = resumed(served, &admission.token, &again, initial.id);
    location(
        &client,
        initial.id,
        &served.origin,
        grant["game_id"].as_str(),
    );
    let pending = recovery::crash(runtime, served, initial.id, &again, &mut agent);
    let restored = sdk_start(&account, &client, initial.id, &served.origin);
    location(
        &client,
        initial.id,
        &served.origin,
        restored["game_id"].as_str(),
    );
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

fn discover(client: &TowerHttp, run: Uuid, gateway: &str) -> Uuid {
    let deadline = Instant::now() + Duration::from_secs(25);
    loop {
        let Response::Hosts { hosts, .. } = client
            .exchange(&Request::Hosts { run, before: None })
            .unwrap()
        else {
            panic!("global host discovery");
        };
        if let Some(host) = hosts.first() {
            assert_eq!(hosts.len(), 1);
            assert_eq!(host.origin, gateway);
            return host.id;
        }
        assert!(
            Instant::now() < deadline,
            "background heartbeat discovers an agent without a player gateway request"
        );
        std::thread::sleep(Duration::from_millis(500));
    }
}

fn busy(served: &Served, client: &TowerHttp, run: Uuid, token: &str) {
    let (status, body) = common::http(served.gateway.port, "GET", "/tower", Some(token), "");
    assert_eq!(status, 200);
    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["engines"], serde_json::json!([]));
    assert!(
        matches!(client.exchange(&Request::Hosts { run, before: None }).unwrap(), Response::Hosts { hosts, .. } if hosts.is_empty())
    );
}

fn player(runtime: &Runtime, served: &Served, name: &str) -> baylee_account::Account {
    fresh::register(runtime, &served.service, &served.endpoint, name, br#"{"version":1,"gateways":[],"preferences":{},"legacy":null,"chat":{"devices":[],"installations":[]}}"#)
}

fn resumed(
    served: &Served,
    token: &str,
    previous: &serde_json::Value,
    run: Uuid,
) -> serde_json::Value {
    let path = format!(
        "/lobby/games/{}/seat",
        previous["game_id"].as_str().unwrap()
    );
    let (status, body) = common::http(served.gateway.port, "POST", &path, Some(token), "");
    assert_eq!(status, 200);
    let granted: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(granted["tower"], serde_json::json!(run.as_bytes()));
    assert!(granted.get("challenge").is_none());
    assert_eq!(granted["game_id"], previous["game_id"]);
    assert_eq!(granted["seat"], 0);
    assert_ne!(granted["seat_token"], previous["seat_token"]);
    granted
}

fn sdk_start(
    account: &baylee_account::Account,
    client: &TowerHttp,
    run: Uuid,
    origin: &str,
) -> serde_json::Value {
    let play = account.play_tower(client, run).unwrap();
    assert_eq!(play.run, run);
    assert_eq!(play.host.origin, origin);
    serde_json::json!({ "game_id": play.seat.game_id, "seat": play.seat.seat, "seat_token": play.seat.seat_token.as_str() })
}

fn location(client: &TowerHttp, run: Uuid, origin: &str, game: Option<&str>) {
    let Response::Location { location } = client.exchange(&Request::Locate { run }).unwrap() else {
        panic!("fresh owned routing");
    };
    assert_eq!(location.run, run);
    assert_eq!(location.host.unwrap().origin, origin);
    assert_eq!(location.game.map(|id| id.to_string()).as_deref(), game);
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
    assert!(foreign.play_tower(client, reserved.id).is_err());
    assert!(
        foreign
            .play_tower(&foreign.tower().unwrap(), reserved.id)
            .is_err()
    );
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

fn shop(client: &TowerHttp, initial: &Run) -> Run {
    let offer = initial
        .shop
        .offers
        .iter()
        .find(|offer| matches!(offer.goods, baylee_tower::v2::shop::Goods::Relic(_)))
        .unwrap();
    let bought = owned(
        client,
        &Request::Command {
            request: Uuid::now_v7(),
            run: initial.id,
            revision: initial.revision,
            command: Command::Buy {
                offer: offer.id,
                delivery: baylee_tower::v2::shop::Delivery::Inventory,
            },
        },
    );
    let equipped = owned(
        client,
        &Request::Command {
            request: Uuid::now_v7(),
            run: bought.id,
            revision: bought.revision,
            command: Command::Move {
                item: bought.economy.inventory.relics[0].id,
                slot: baylee_tower::v2::inventory::Slot::Active(0),
            },
        },
    );
    owned(
        client,
        &Request::Command {
            request: Uuid::now_v7(),
            run: equipped.id,
            revision: equipped.revision,
            command: Command::Freeze { frozen: true },
        },
    )
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
    assert_eq!(settled.phase, Phase::Reward);
    assert_eq!(settled.history[0].outcome, Outcome::Defeat);
    let bonus: u16 = initial
        .economy
        .inventory
        .active()
        .iter()
        .filter(|item| item.family == baylee_tower::v2::catalog::Family::GildedAcorn)
        .map(|item| u16::from(item.rarity.tier()))
        .sum();
    assert_eq!(settled.history[0].bonus_gold, bonus);
    assert_eq!(
        settled.economy.gold,
        initial.economy.gold + DUEL_INCOME + bonus
    );
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
    let reward = settled.reward.as_ref().unwrap();
    assert_eq!(reward.take, 1);
    assert_eq!(reward.cards.len(), 5);
    let claim = Request::Command {
        request: Uuid::now_v7(),
        run: settled.id,
        revision: settled.revision,
        command: Command::Claim {
            cards: vec![reward.cards[0]],
        },
    };
    let next = owned(client, &claim);
    assert_eq!(next.phase, Phase::Shop);
    assert_eq!(next.shop.stage, 2);
    assert!(next.shop.frozen);
    for (old, new) in initial.shop.offers.iter().zip(&next.shop.offers) {
        if old.sold {
            assert_ne!(old.id, new.id);
        } else {
            assert_eq!(old, new);
        }
    }
    assert_eq!(
        owned(client, &claim),
        next,
        "reward retry cannot duplicate gold or reroll frozen stock"
    );
}
