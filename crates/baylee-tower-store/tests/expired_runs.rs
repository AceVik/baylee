//! A run left until it expired gives its cohort's one active place back.
#![forbid(unsafe_code)]
mod support;

use baylee_tower_store::{CONSENT_REVISION, Error, Start, Store};
use sea_orm::{ConnectionTrait, DatabaseConnection};
use support::{Sandbox, publication};
use uuid::Uuid;

#[tokio::test]
async fn a_run_left_past_its_expiry_no_longer_blocks_its_cohort() {
    let sandbox = Sandbox::open().await;
    let (store, db, schema) = (
        sandbox.store.clone(),
        sandbox.db.clone(),
        sandbox.schema.clone(),
    );
    let observed = tokio::spawn(async move { story(store, db, schema).await }).await;
    sandbox.close().await;
    observed.expect("expiry scenario completed without assertion failure");
}

async fn story(store: Store, db: DatabaseConnection, schema: String) {
    let cohort = store.publish(&publication()).await.unwrap();
    let account = Uuid::now_v7();
    let choice = Start {
        cohort,
        starter: 0,
        consent: CONSENT_REVISION,
    };
    let first = store
        .start(account, Uuid::now_v7(), choice, [7; 32])
        .await
        .unwrap();
    // While it lives, it is the cohort's one run.
    assert_eq!(
        store.start(account, Uuid::now_v7(), choice, [8; 32]).await,
        Err(Error::Conflict)
    );
    db.execute_unprepared(&format!(
        "UPDATE \"{schema}\".runs SET expires_at=pg_catalog.clock_timestamp()-interval '1 second' WHERE id='{}'",
        first.id
    ))
    .await
    .unwrap();
    // Expired, its owner can no longer read it, and so could never end it.
    assert_eq!(store.get(account, first.id).await, Err(Error::NotFound));
    let second = store
        .start(account, Uuid::now_v7(), choice, [9; 32])
        .await
        .unwrap();
    assert_ne!(second.id, first.id);
    assert_eq!(store.get(account, second.id).await.unwrap(), second);
    // The new run is the one active run again.
    assert_eq!(
        store.start(account, Uuid::now_v7(), choice, [10; 32]).await,
        Err(Error::Conflict)
    );
}
