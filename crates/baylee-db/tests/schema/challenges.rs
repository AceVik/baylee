//! Account binding, once-only launch, durable delivery and deletion on real PostgreSQL.
use super::{Sandbox, Uuid, an_account};
use baylee_db::challenges as store;
use central_api::challenges::{AttemptState, Completion};
use sea_orm::{ActiveModelTrait, EntityTrait};

#[tokio::test]
async fn challenge_claims_launch_once_and_results_survive_retries_without_changing() {
    let sandbox = Sandbox::open("challenge_outbox").await;
    let db = &sandbox.db;
    let account = an_account("challenge@example.org")
        .insert(db)
        .await
        .unwrap()
        .id;
    let central = Uuid::now_v7();
    let attempt = Uuid::now_v7();
    let game = Uuid::now_v7();
    assert_eq!(
        store::reserve(db, attempt, game, account, central)
            .await
            .unwrap(),
        game
    );
    assert_eq!(
        store::reserve(db, attempt, Uuid::now_v7(), account, central)
            .await
            .unwrap(),
        game
    );
    assert!(
        store::reserve(db, attempt, Uuid::now_v7(), account, Uuid::now_v7())
            .await
            .is_err()
    );
    let (one, two) = tokio::join!(
        store::launch_once(db, attempt, game),
        store::launch_once(db, attempt, game)
    );
    assert_ne!(one.unwrap(), two.unwrap(), "a game is never launched twice");
    store::discard_unstarted(db, attempt).await.unwrap();
    assert_eq!(
        store::pending(db).await.unwrap().len(),
        1,
        "launched claims cannot be discarded"
    );
    let result = Completion {
        attempt: *attempt.as_bytes(),
        game: *game.as_bytes(),
        state: AttemptState::Won,
        turn: 3,
        finished_at: 1_800_000_000,
        milliseconds: Some(23_456),
        record_digest: [7; 32],
    };
    store::complete(db, &result).await.unwrap();
    store::complete(db, &result).await.unwrap();
    let mut conflict = result.clone();
    conflict.turn = 2;
    assert!(store::complete(db, &conflict).await.is_err());
    assert_eq!(store::pending(db).await.unwrap()[0].result, Some(result));
    assert!(!store::launch_once(db, attempt, game).await.unwrap());
    store::touch(db, attempt).await.unwrap();
    store::delivered(db, attempt).await.unwrap();
    assert!(store::pending(db).await.unwrap().is_empty());
    let first = Uuid::now_v7();
    store::reserve(db, first, Uuid::now_v7(), account, central)
        .await
        .unwrap();
    store::reserve(db, Uuid::now_v7(), Uuid::now_v7(), account, central)
        .await
        .unwrap();
    assert!(
        store::reserve(db, Uuid::now_v7(), Uuid::now_v7(), account, central)
            .await
            .is_err(),
        "bounded unresolved claims per account"
    );
    store::discard_unstarted(db, first).await.unwrap();
    assert_eq!(store::pending(db).await.unwrap().len(), 1);
    baylee_db::entity::account::Entity::delete_by_id(account)
        .exec(db)
        .await
        .unwrap();
    assert!(
        store::pending(db).await.unwrap().is_empty(),
        "account deletion removes private delivery metadata"
    );
    sandbox.close().await;
}
