use super::transport::{Central, Client};
use baylee_tower_api::{
    Request, Response,
    ranking::{Board, Lane, Listing},
};
use std::sync::atomic::Ordering;
use uuid::Uuid;

pub async fn exercise(client: &Client, central: &Central, cohort: Uuid) {
    let token = &central.tokens[0];
    let read = Request::Ranking {
        cohort,
        lane: Lane::Echoes,
    };
    let answer = client.send(token, &read).await;
    assert_eq!(answer.0, 200);
    let Response::Ranking { board } = serde_json::from_slice(&answer.1).unwrap() else {
        panic!("ranking");
    };
    assert_eq!(
        board,
        Board {
            cohort,
            lane: Lane::Echoes,
            listing: Listing::default(),
            best: None,
            leaders: Vec::new()
        }
    );
    let publish = Request::Listing {
        revision: 0,
        listed: true,
    };
    let published = client.send(token, &publish).await;
    assert_eq!(published.0, 200);
    assert_eq!(published, client.send(token, &publish).await, "exact retry");
    assert_eq!(
        client.send(&central.tokens[1], &publish).await.0,
        404,
        "identity cannot select another owner"
    );
    let forged =
        br#"{"operation":"listing","revision":1,"listed":true,"display_name":"Victim"}"#.to_vec();
    let calls = central.calls.load(Ordering::SeqCst);
    assert_eq!(client.raw(Some(token), forged, &[]).await.0, 400);
    assert_eq!(central.calls.load(Ordering::SeqCst), calls);
    central.status.store(401, Ordering::SeqCst);
    assert_eq!(
        client.send(token, &read).await.0,
        401,
        "every board read rechecks revocation"
    );
    assert_eq!(
        client
            .send(
                token,
                &Request::Listing {
                    revision: 1,
                    listed: false
                }
            )
            .await
            .0,
        401
    );
    central.status.store(200, Ordering::SeqCst);
    let hidden = client
        .send(
            token,
            &Request::Listing {
                revision: 1,
                listed: false,
            },
        )
        .await;
    assert_eq!(hidden.0, 200);
    assert_eq!(
        client.send(token, &publish).await.0,
        409,
        "old publication cannot undo withdrawal"
    );
}
