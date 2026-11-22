//! Real authenticated gateway, migrated PostgreSQL and two socket seats at one
//! actual engine. Only the external Central service is replaced by a wire peer.
use crate::{common, stub_central};
use baylee_catalog::{Catalog, scryfall};
use baylee_protocol::v1::{self, Envelope, envelope::Msg};
use central_api::ranked::Outcome;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread")]
async fn ranked_duel_claims_both_seats_and_delivers_after_retry() {
    let stub = stub_central::Stub::serve(Duration::ZERO);
    let (gw, _settings) = stub_central::gateway(&stub, "ranked_duel");
    let accounts = [1, 2, 3].map(|key| stub_central::linked(&gw, &stub, key));
    let catalog = Catalog::connect(&gw.database_url()).await.unwrap();
    let forest = baylee_cards::by_index(baylee_cards::decks::by_name("Forest").unwrap()).unwrap();
    catalog
        .upsert(&[scryfall::Card {
            id: uuid::Uuid::now_v7().to_string(),
            oracle_id: Some(forest.oracle_id.into()),
            lang: "en".into(),
            set: "tst".into(),
            collector_number: "1".into(),
            name: "Forest".into(),
            type_line: Some("Basic Land — Forest".into()),
            legalities: [("standard".into(), "legal".into())].into(),
            ..Default::default()
        }])
        .await
        .unwrap();
    catalog.bump_data_version().await.unwrap();
    let _agent = common::attach_agent(&gw).await;
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
    assert_eq!(
        gw.scalar("SELECT count(*) FROM ranked_run WHERE launched"),
        1
    );
    assert_eq!(
        common::http(
            gw.port,
            "POST",
            &format!("/lobby/games/{game}/seat"),
            Some(&accounts[2]),
            "{}"
        )
        .0,
        403
    );
    let mut first = common::dial_seat(gw.port, game, common::json_field(&a, "seat_token")).await;
    let mut second = common::dial_seat(gw.port, game, common::json_field(&b, "seat_token")).await;
    for socket in [&mut first, &mut second] {
        common::send(
            socket,
            &Envelope {
                msg: Some(Msg::SeatReady(v1::SeatReady {})),
            },
        )
        .await;
    }
    tokio::time::timeout(common::WAIT_BUDGET, async {
        while let Some(msg) = common::next_msg(&mut first).await {
            if matches!(msg, Msg::ChoiceRequest(_)) {
                return;
            }
        }
        panic!("ranked seat closed before first decision");
    })
    .await
    .unwrap();
    let (out, message) = common::http(gw.port, "POST", "/lobby/depart", Some(&accounts[0]), "{}");
    assert_eq!(out, 204, "{message}");
    tokio::time::timeout(Duration::from_secs(40), async {
        while stub.ranked_result().is_none() || gw.scalar("SELECT count(*) FROM ranked_run") != 0 {
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
    })
    .await
    .expect("the outbox retries after a refused delivery");
    assert_eq!(stub.ranked_result(), Some((Outcome::SecondWins, 2)));
}
