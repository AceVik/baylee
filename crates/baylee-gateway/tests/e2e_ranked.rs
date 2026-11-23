//! Real authenticated gateway, migrated PostgreSQL and two socket seats at one
//! actual engine. Only the external Central service is replaced by a wire peer.
use crate::{common, stub_central};
use baylee_protocol::v1::{self, Envelope, envelope::Msg};
use central_api::ranked::Outcome;
use std::time::Duration;

#[test]
fn relay_seats_have_separate_reconnect_allowances_behind_one_address() {
    let stub = stub_central::Stub::serve(Duration::ZERO);
    let (gw, _settings) = stub_central::gateway(&stub, "ranked_budget");
    stub.knows(1);
    stub.knows(2);
    let id = stub.reserve_ranked([1, 2], [[0; 32]; 2]);
    let first = stub.game_login(gw.port, 1, id);
    let second = stub.game_login(gw.port, 2, id);
    let admit =
        |credential: &str| common::http(gw.port, "POST", "/auth/ranked/login", None, credential).0;
    let mut forged: serde_json::Value = serde_json::from_str(&first).unwrap();
    let mut token = forged["token"].as_str().unwrap().to_owned();
    let signature = token.rfind('.').unwrap() + 1;
    let replacement = if token.as_bytes()[signature] == b'A' {
        "B"
    } else {
        "A"
    };
    token.replace_range(signature..signature + 1, replacement);
    forged["token"] = token.into();
    assert_eq!(admit(&forged.to_string()), 401);
    for _ in 0..12 {
        assert_eq!(admit(&first), 200);
    }
    assert_eq!(admit(&first), 429);
    assert_eq!(admit(&second), 200);
    assert_eq!(stub.sign_in(gw.port, 1).0, 200);
}

#[tokio::test(flavor = "multi_thread")]
async fn ranked_duel_claims_both_seats_and_delivers_after_retry() {
    let stub = stub_central::Stub::serve(Duration::ZERO);
    let (gw, _settings) = stub_central::gateway(&stub, "ranked_duel");
    let home = [1, 2, 3].map(|key| stub_central::linked(&gw, &stub, key));
    let _agent = common::ranked::provision(&gw).await;
    tokio::time::timeout(Duration::from_secs(40), async {
        while stub.ranked_capacity() == 0 {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("catalog and compatible engine capacity advertised to Central");
    let reg =
        baylee_cards::ranked::registration("standard", &["60 Forest".into()], &[], &[]).unwrap();
    let id = stub.reserve_ranked([1, 2], [reg.digest; 2]);
    let local_accounts = gw.scalar("SELECT count(*) FROM account");
    let accounts = [1, 2, 3].map(|key| stub.game_bearer(gw.port, key, id));
    assert_eq!(
        gw.scalar("SELECT count(*) FROM account"),
        local_accounts,
        "match aliases must never become ordinary local accounts"
    );
    assert_scoped_access(gw.port, &accounts[0], &home[0]);
    let path = format!("/ranked/{id}/play");
    let body=serde_json::json!({"format":"standard","cards":["60 Forest"],"sideboard":[],"commanders":[]}).to_string();
    let claim =
        |seat: usize, deck: &str| common::http(gw.port, "POST", &path, Some(&accounts[seat]), deck);
    let (out, _) = claim(2, &body);
    assert_eq!(out, 409, "Central refuses the third account");
    let changed = body.replace("60 Forest", "61 Forest");
    assert_eq!(claim(0, &changed).0, 409, "cannot change the queued deck");
    let (out, message) = claim(0, &body);
    assert_eq!(out, 202, "{message}");
    let (out, b) = claim(1, &body);
    assert_eq!(out, 200, "{b}");
    let (out, a) = claim(0, &body);
    assert_eq!(out, 200, "{a}");
    let game = common::json_field(&a, "game_id");
    assert_eq!(game, common::json_field(&b, "game_id"));
    let (status, chairs) = common::http(gw.port, "GET", "/lobby/seats", Some(&home[0]), "");
    assert_eq!(status, 200);
    assert!(
        !chairs.contains(game),
        "ranked recovery belongs to Central status"
    );
    assert_eq!(
        gw.scalar("SELECT count(*) FROM ranked_run WHERE launched"),
        1
    );
    assert_eq!(
        common::http(
            gw.port,
            "POST",
            &format!("/lobby/games/{game}/seat"),
            Some(&home[2]),
            "{}"
        )
        .0,
        403
    );
    let (out, reissued) = claim(0, &serde_json::json!({"game":game}).to_string());
    assert_eq!(
        out, 200,
        "the same dedicated host session can reclaim its own game"
    );
    let mut first =
        common::dial_seat(gw.port, game, common::json_field(&reissued, "seat_token")).await;
    let mut second = common::dial_seat(gw.port, game, common::json_field(&b, "seat_token")).await;
    depart_lobby_and_concede(gw.port, &home[1], &mut first, &mut second).await;
    tokio::time::timeout(Duration::from_secs(40), async {
        while stub.ranked_result().is_none() || gw.scalar("SELECT count(*) FROM ranked_run") != 0 {
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
    })
    .await
    .expect("the outbox retries after a refused delivery");
    assert_eq!(stub.ranked_result(), Some((Outcome::SecondWins, 2)));
    stub.revoke(65);
    stub.push(gw.port);
    assert_eq!(claim(0, &body).0, 401);
    assert_eq!(
        common::http(gw.port, "GET", "/me", Some(&home[0]), "").0,
        200
    );
}

fn assert_scoped_access(port: u16, scoped: &str, home: &str) {
    for path in ["/me", "/decks", "/lobby/games"] {
        assert_eq!(common::http(port, "GET", path, Some(scoped), "").0, 401);
    }
    assert_eq!(
        common::http(port, "POST", "/auth/session/renew", Some(scoped), "{}").0,
        401
    );
    let other = format!("/ranked/{}/play", uuid::Uuid::now_v7());
    assert_eq!(
        common::http(
            port,
            "POST",
            &other,
            Some(scoped),
            r#"{"game":"019afcb6-1234-7001-8421-000000000000"}"#
        )
        .0,
        401
    );
    assert_eq!(common::http(port, "GET", "/me", Some(home), "").0, 200);
}

async fn depart_lobby_and_concede(
    port: u16,
    home: &str,
    first: &mut common::Socket,
    second: &mut common::Socket,
) {
    for socket in [&mut *first, second] {
        common::send(
            socket,
            &Envelope {
                msg: Some(Msg::SeatReady(v1::SeatReady {})),
            },
        )
        .await;
    }
    tokio::time::timeout(common::WAIT_BUDGET, async {
        while let Some(msg) = common::next_msg(first).await {
            if matches!(msg, Msg::ChoiceRequest(_)) {
                return;
            }
        }
        panic!("ranked seat closed before first decision");
    })
    .await
    .unwrap();
    assert_eq!(
        common::http(port, "POST", "/lobby/depart", Some(home), "").0,
        204,
        "leaving the home lobby must preserve the second ranked seat"
    );
    common::send(
        first,
        &Envelope {
            msg: Some(Msg::PlayerAction(v1::PlayerActionMsg {
                game_id: String::new(),
                seat_token: String::new(),
                action: baylee_core::wire::encode(&baylee_engine::choice::PlayerAction::Concede),
            })),
        },
    )
    .await;
}
