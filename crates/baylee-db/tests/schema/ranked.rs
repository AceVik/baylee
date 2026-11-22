//! The assigned host must recover exactly the same game and result after reopening its store.
use super::{Sandbox, Uuid};
use baylee_db::{ranked, records};
use central_api::ranked::Outcome;

#[tokio::test]
async fn ranked_binding_and_terminal_evidence_survive_a_new_connection() {
    let sandbox = Sandbox::open("ranked_resume").await;
    let db = &sandbox.db;
    let id = Uuid::now_v7();
    let (first, second) = tokio::join!(ranked::reserve(db, id), ranked::reserve(db, id));
    let game = first.unwrap();
    assert_eq!(second.unwrap(), game);
    let (a, b) = tokio::join!(
        ranked::launch_once(db, id, game),
        ranked::launch_once(db, id, game)
    );
    assert_ne!(a.unwrap(), b.unwrap());
    let game_id = game.to_string();
    records::open(db, &game_id, &[]).await.unwrap();
    records::append(db, &game_id, 0, vec![1], false)
        .await
        .unwrap();
    records::finish(db, &game_id, &[0], Some(3), "conceded")
        .await
        .unwrap();
    assert!(ranked::pending(db).await.unwrap()[0].outcome.is_none());
    records::append(db, &game_id, 1, vec![2], true)
        .await
        .unwrap();
    let reopened = baylee_db::open(&sandbox.scoped, 1).await.unwrap();
    let queued = ranked::pending(&reopened).await.unwrap();
    assert_eq!(queued.len(), 1);
    assert_eq!(
        (queued[0].match_id, queued[0].game, queued[0].outcome),
        (id, game, Some(Outcome::FirstWins))
    );
    assert!(!ranked::launch_once(&reopened, id, game).await.unwrap());
    ranked::delivered(&reopened, id).await.unwrap();
    assert!(ranked::pending(db).await.unwrap().is_empty());
    reopened.close().await.unwrap();
    sandbox.close().await;
}
