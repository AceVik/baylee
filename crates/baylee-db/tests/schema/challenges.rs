//! Account binding, once-only launch, durable delivery and deletion on real PostgreSQL.
use super::{Sandbox, Uuid, an_account};
use baylee_db::challenges as store;
use central_api::challenges::{AttemptState, Completion};
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Statement};

#[tokio::test]
async fn ownership_never_consumes_the_only_request_connection() {
    let sandbox = Sandbox::open("challenge_owner_pool").await;
    let requests = baylee_db::open(&sandbox.scoped, 1).await.unwrap();
    let mut owner = store::Owner::acquire(&requests).await.unwrap().unwrap();
    owner.check().await.unwrap();
    let row = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        requests.query_one_raw(Statement::from_string(
            requests.get_database_backend(),
            "SELECT count(*) AS accounts FROM account",
        )),
    )
    .await
    .expect("ownership must leave the single request connection available")
    .unwrap()
    .unwrap();
    assert_eq!(row.try_get::<i64>("", "accounts").unwrap(), 0);
    // The copied connection options preserve this schema, so a second pool
    // cannot acquire an independent lock for the same challenge runtime.
    assert!(store::Owner::acquire(&sandbox.db).await.unwrap().is_none());
    owner.release().await.unwrap();
    requests.close().await.unwrap();
    sandbox.close().await;
}

#[tokio::test]
async fn only_one_gateway_can_reconcile_games_in_a_schema() {
    let sandbox = Sandbox::open("challenge_owner").await;
    let mut owner = store::Owner::acquire(&sandbox.db).await.unwrap().unwrap();
    owner.check().await.unwrap();
    assert!(store::Owner::acquire(&sandbox.db).await.unwrap().is_none());
    owner.release().await.unwrap();
    let mut next = store::Owner::acquire(&sandbox.db).await.unwrap().unwrap();
    next.check().await.unwrap();
    next.release().await.unwrap();
    sandbox.close().await;
}

#[tokio::test]
async fn a_lost_owner_connection_is_detected_and_its_lock_released() {
    let sandbox = Sandbox::open("challenge_owner_lost").await;
    let db = &sandbox.db;
    let mut owner = store::Owner::acquire(db).await.unwrap().unwrap();
    // Terminate only the connection holding this isolated schema's own test lock.
    let row = db.query_one_raw(Statement::from_string(db.get_database_backend(),
        "SELECT pg_terminate_backend(pid) AS stopped FROM pg_locks WHERE locktype = 'advisory' AND granted AND classid = hashtext(current_database())::oid AND objid = hashtext(current_schema() || '/baylee/challenges/v1')::oid AND pid <> pg_backend_pid()"))
        .await.unwrap().unwrap();
    assert!(row.try_get::<bool>("", "stopped").unwrap());
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(3), owner.check())
            .await
            .unwrap()
            .is_err()
    );
    drop(owner);
    store::Owner::acquire(db)
        .await
        .unwrap()
        .unwrap()
        .release()
        .await
        .unwrap();
    sandbox.close().await;
}

#[tokio::test]
async fn rotating_young_reservations_does_not_restart_their_recovery_grace() {
    let sandbox = Sandbox::open("challenge_queue_rotation").await;
    let db = &sandbox.db;
    let mut attempts = Vec::new();
    for index in 0..33 {
        let account = an_account(&format!("queue-{index}@example.org"))
            .insert(db)
            .await
            .unwrap()
            .id;
        let attempt = Uuid::now_v7();
        store::reserve(db, attempt, Uuid::now_v7(), account, Uuid::now_v7())
            .await
            .unwrap();
        attempts.push(attempt);
    }
    db.execute_raw(Statement::from_string(
        db.get_database_backend(),
        "UPDATE challenge_run SET created_at = created_at - interval '5 minutes'",
    ))
    .await
    .unwrap();
    let first = store::pending(db).await.unwrap();
    assert_eq!(first.len(), 32);
    let omitted = *attempts
        .iter()
        .find(|id| !first.iter().any(|run| run.attempt == **id))
        .unwrap();
    for run in first {
        store::touch(db, run.attempt).await.unwrap();
    }
    let next = store::pending(db).await.unwrap();
    assert_eq!(
        next[0].attempt, omitted,
        "the 33rd reservation cannot starve behind the first batch"
    );
    assert!(
        next.iter().all(|run| run.age >= 300),
        "queue rotation must not renew recovery grace"
    );
    sandbox.close().await;
}

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
