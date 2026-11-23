//! The assigned host must recover exactly the same game and result after reopening its store.
use super::{Sandbox, Uuid};
use baylee_db::{ranked, records};
use central_api::ranked::Outcome;
use sea_orm::ConnectionTrait;

#[tokio::test]
async fn replicated_placements_reject_stale_roles_and_legacy_delivery() {
    let sandbox = Sandbox::open("ranked_roles").await;
    let db = &sandbox.db;
    let game = Uuid::now_v7();
    assert!(
        ranked::replica::bind(db, game, game, 1, false)
            .await
            .unwrap()
    );
    assert!(
        !ranked::replica::bind(db, game, Uuid::now_v7(), 2, true)
            .await
            .unwrap()
    );
    assert!(
        !ranked::replica::bind(db, game, game, 1, true)
            .await
            .unwrap()
    );
    assert!(ranked::reserve(db, game).await.is_err());
    assert!(!ranked::launch_once(db, game, game).await.unwrap());
    ranked::delivered(db, game).await.unwrap();
    assert!(ranked::pending(db).await.unwrap().is_empty());
    let reopened = baylee_db::open(&sandbox.scoped, 1).await.unwrap();
    assert!(
        ranked::replica::bind(&reopened, game, game, 2, true)
            .await
            .unwrap()
    );
    assert!(
        !ranked::replica::bind(db, game, game, 1, false)
            .await
            .unwrap()
    );
    assert!(!ranked::replica::finish(db, game, game, 1).await.unwrap());
    let orphan = Uuid::now_v7();
    assert!(
        ranked::replica::bind(db, orphan, orphan, 1, true)
            .await
            .unwrap()
    );
    db.execute_unprepared("UPDATE ranked_run SET seen_at=clock_timestamp()-interval '2 days'")
        .await
        .unwrap();
    ranked::replica::reap_absent(db, &[game]).await.unwrap();
    assert!(
        !ranked::replica::finish(db, orphan, orphan, 1)
            .await
            .unwrap()
    );
    assert!(ranked::replica::finish(db, game, game, 2).await.unwrap());
    reopened.close().await.unwrap();
    sandbox.close().await;
}

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
