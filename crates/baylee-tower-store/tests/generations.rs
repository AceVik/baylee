//! Retained generation-one state and receipts survive catalog preparation unchanged.
#![forbid(unsafe_code)]
mod support;

use baylee_tower::v2::{package::Cohort, run::Command};
use baylee_tower_store::{CONSENT_REVISION, Start};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use support::{Sandbox, publication};
use uuid::Uuid;

#[tokio::test]
async fn legacy_state_retries_and_refreshes_keep_the_published_generation() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let observed = tokio::spawn(async move {
        let mut publication = publication();
        publication.cohort =
            Cohort::for_generation(publication.cohort.executor.clone(), &publication.catalog, 1)
                .unwrap();
        let cohort = store.publish(&publication).await.unwrap();
        let account = Uuid::now_v7();
        let request = Uuid::now_v7();
        let choice = Start {
            cohort,
            starter: 0,
            consent: CONSENT_REVISION,
        };
        let first = store
            .start(account, request, choice, [7; 32])
            .await
            .unwrap();
        let original = serde_json::to_vec(&first).unwrap();
        let mut legacy = serde_json::to_value(&first.state).unwrap();
        assert!(legacy["shop"].get("generator").is_none());
        assert_eq!(first.state.shop.generator, 1);
        // Explicitly persist the legacy encoding, not a structure with a new field.
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            format!("UPDATE \"{schema}\".runs SET state=$1 WHERE id=$2"),
            [serde_json::to_vec(&legacy).unwrap().into(), first.id.into()],
        ))
        .await
        .unwrap();
        assert_eq!(store.get(account, first.id).await.unwrap(), first);
        assert_eq!(
            store.runs(account, None, 8).await.unwrap(),
            std::slice::from_ref(&first)
        );
        let refreshed = store
            .command(account, Uuid::now_v7(), first.id, 0, &Command::Refresh {})
            .await
            .unwrap();
        assert_eq!(refreshed.state.shop.generator, 1);
        assert_eq!(refreshed.state.shop.rerolls, 1);
        assert_eq!(refreshed.state.economy.gold, first.state.economy.gold - 2);
        let retried = store
            .start(account, request, choice, [99; 32])
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_vec(&retried).unwrap(),
            original,
            "the original receipt stays byte-identical after subsequent changes"
        );
        assert_eq!(store.get(account, first.id).await.unwrap(), refreshed);

        legacy = serde_json::to_value(&refreshed.state).unwrap();
        legacy["shop"]["generator"] = u16::MAX.into();
        let corrupt = serde_json::to_vec(&legacy).unwrap();
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            format!("UPDATE \"{schema}\".runs SET state=$1 WHERE id=$2"),
            [corrupt.clone().into(), first.id.into()],
        ))
        .await
        .unwrap();
        assert!(store.get(account, first.id).await.is_err());
        assert!(store.runs(account, None, 8).await.is_err());
        assert!(
            store
                .command(account, Uuid::now_v7(), first.id, 1, &Command::Refresh {})
                .await
                .is_err()
        );
        let row = db
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                format!("SELECT state,revision FROM \"{schema}\".runs WHERE id=$1"),
                [first.id.into()],
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<Vec<u8>>("", "state").unwrap(), corrupt);
        assert_eq!(row.try_get::<i64>("", "revision").unwrap(), 1);
    })
    .await;
    sandbox.close().await;
    observed.expect("generation retention and corruption scenario completed");
}
