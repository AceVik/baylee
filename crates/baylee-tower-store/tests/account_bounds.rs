//! Migration of existing state and transactional quota accounting on owned schemas.
#![forbid(unsafe_code)]
mod support;

use baylee_tower::{
    Refusal,
    v2::{run::Command, shop::Delivery},
};
use baylee_tower_store::{CONSENT_REVISION, Error, MAX_RECEIPT_BYTES, MAX_RUNS, Start};
use sea_orm::ConnectionTrait;
use support::{Sandbox, publication};
use uuid::Uuid;

#[tokio::test]
async fn existing_history_upgrades_and_quota_failures_leave_no_partial_mutation() {
    let sandbox = Sandbox::open().await;
    let db = sandbox.db.clone();
    let store = sandbox.store.clone();
    let schema = sandbox.schema.clone();
    let observed = tokio::spawn(async move {
        let cohort = store.publish(&publication()).await.unwrap();
        let account = Uuid::now_v7();
        let choice = Start { cohort, starter: 0, consent: CONSENT_REVISION };
        let initial = store.start(account, Uuid::now_v7(), choice, [9; 32]).await.unwrap();
        let request = Uuid::now_v7();
        let buy = Command::Buy { offer: initial.state.shop.offers[0].id, delivery: Delivery::Inventory };
        let bought = store.command(account, request, initial.id, 0, &buy).await.unwrap();
        // Restore exactly the first migration's schema around valid saved data;
        // only this fixture's uniquely named schema is ever modified.
        db.execute_unprepared(&format!("ALTER TABLE \"{schema}\".players DROP COLUMN consent_revision,
            DROP COLUMN consent_at,DROP COLUMN receipt_bytes,DROP COLUMN run_count;
            DELETE FROM \"{schema}\".schema_version WHERE version=2")).await.unwrap();
        assert_eq!(store.ready().await, Err(Error::Schema));
        store.migrate().await.unwrap();
        store.ready().await.unwrap();
        store.migrate().await.unwrap();
        assert_eq!(store.get(account, initial.id).await.unwrap(), bought);
        let row = db.query_one_raw(sea_orm::Statement::from_string(sea_orm::DbBackend::Postgres,
            format!("SELECT p.receipt_bytes=(SELECT sum(octet_length(response)) FROM \"{schema}\".requests) AS bytes_ok,
                p.run_count=1 AS runs_ok,p.consent_revision=1 AND p.consent_at IS NOT NULL AS consent_ok FROM \"{schema}\".players p"))).await.unwrap().unwrap();
        for field in ["bytes_ok", "runs_ok", "consent_ok"] { assert!(row.try_get::<bool>("", field).unwrap()); }

        db.execute_unprepared(&format!("UPDATE \"{schema}\".players SET receipt_bytes={MAX_RECEIPT_BYTES}")).await.unwrap();
        assert_eq!(store.command(account, Uuid::now_v7(), initial.id, 1, &Command::Refresh).await, Err(Error::Refused(Refusal::Capacity)));
        assert_eq!(store.get(account, initial.id).await.unwrap(), bought);
        assert_eq!(store.command(account, request, initial.id, 0, &buy).await.unwrap(), bought,
            "a committed response remains retrievable at the storage limit");
        db.execute_unprepared(&format!("UPDATE \"{schema}\".players SET receipt_bytes=(SELECT sum(octet_length(response)) FROM \"{schema}\".requests),run_count={MAX_RUNS}")).await.unwrap();
        store.command(account, Uuid::now_v7(), initial.id, 1, &Command::Abandon).await.unwrap();
        let next_request = Uuid::now_v7();
        assert_eq!(store.start(account, next_request, choice, [10; 32]).await, Err(Error::Refused(Refusal::Capacity)));
        db.execute_unprepared(&format!("UPDATE \"{schema}\".players SET run_count=(SELECT count(*) FROM \"{schema}\".runs)")).await.unwrap();
        let next = store.start(account, next_request, choice, [10; 32]).await.unwrap();
        assert_eq!(next.state.shop, initial.state.shop);
    }).await;
    sandbox.close().await;
    observed.expect("migration/quota scenario completed without assertion failure");
}
