use super::{Flow, Sandbox};
use baylee_tower::v2::{execution::Prefix, run::Phase};
use baylee_tower_store::{Claimed, ExpiredExecution, MAX_AUTOMATIC_RECOVERIES, Store};
use sea_orm::ConnectionTrait;
use uuid::Uuid;

#[tokio::test]
async fn expired_execution_discovery_rechecks_renewals_and_bounds_automatic_retries() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let observed = tokio::spawn(async move {
        let flow = Flow::prepare(&store).await;
        flow.reserve(&store).await;
        let claim = store
            .claim(&flow.host, flow.initial.id, Uuid::now_v7(), 120_000)
            .await
            .unwrap();
        let prefix = store
            .append(
                &flow.host,
                claim.lease,
                Prefix::default(),
                b"retained operation\n",
            )
            .await
            .unwrap()
            .prefix;
        migration_backfill(&store, &db, &schema).await;
        let fresh = ExpiredExecution {
            run: flow.initial.id,
            fence: claim.lease.fence,
            lease_until: claim.lease_until,
        };
        assert!(
            store
                .recover_expired(&flow.supervisor, fresh)
                .await
                .is_err(),
            "live execution is not a crash"
        );
        assert!(
            store
                .expired_executions(&flow.host, flow.initial.cohort, 1)
                .await
                .is_err()
        );
        assert!(
            store
                .expired_executions(&flow.supervisor, flow.initial.cohort, 0)
                .await
                .is_err()
        );
        assert!(
            store
                .expired_executions(&flow.supervisor, flow.initial.cohort, 1)
                .await
                .unwrap()
                .is_empty()
        );
        retries(&store, &db, &schema, &flow, claim, prefix).await;
    })
    .await;
    sandbox.close().await;
    observed.expect("bounded automatic recovery completed");
}

async fn retries(
    store: &Store,
    db: &sea_orm::DatabaseConnection,
    schema: &str,
    flow: &Flow,
    mut claim: Claimed,
    prefix: Prefix,
) {
    for retry in 0..=MAX_AUTOMATIC_RECOVERIES {
        expire(db, schema, flow.initial.id).await;
        let candidates = store
            .expired_executions(&flow.supervisor, flow.initial.cohort, 1)
            .await
            .unwrap();
        assert_eq!(candidates.len(), 1);
        assert!(
            store
                .recover_expired(&flow.host, candidates[0])
                .await
                .is_err()
        );
        if retry == 0 {
            renewed_hint(store, db, schema, flow, &claim, candidates[0]).await;
        }
        let (first, second) = tokio::join!(
            store.recover_expired(&flow.supervisor, candidates[0]),
            store.recover_expired(&flow.supervisor, candidates[0])
        );
        assert_eq!(
            usize::from(first.is_ok()) + usize::from(second.is_ok()),
            1,
            "only one supervisor owns this recovery"
        );
        let recovered = first.or(second).unwrap();
        assert_eq!(recovered.state.economy, flow.initial.state.economy);
        assert!(recovered.state.history.is_empty());
        let retained = recovered.state.execution.as_ref().unwrap();
        assert_eq!(retained.prefix, prefix);
        assert!(retained.fence > claim.lease.fence);
        assert!(
            store
                .append(&flow.host, claim.lease, prefix, b"stale worker\n")
                .await
                .is_err()
        );
        assert!(
            store
                .expired_executions(&flow.supervisor, flow.initial.cohort, 1)
                .await
                .unwrap()
                .is_empty()
        );
        if retry == MAX_AUTOMATIC_RECOVERIES {
            assert_eq!(recovered.state.phase, Phase::Quarantined);
            assert!(
                store
                    .claim(&flow.host, flow.initial.id, Uuid::now_v7(), 120_000)
                    .await
                    .is_err()
            );
        } else {
            assert_eq!(recovered.state.phase, Phase::Reserved);
            let next = store
                .claim(&flow.host, flow.initial.id, Uuid::now_v7(), 120_000)
                .await
                .unwrap();
            assert!(next.package == claim.package);
            assert_eq!(next.prefix, prefix);
            assert_eq!(next.byte_limit, claim.byte_limit);
            assert_eq!(
                store
                    .read_chunk(&flow.host, next.lease, 0)
                    .await
                    .unwrap()
                    .data,
                b"retained operation\n"
            );
            claim = next;
        }
    }
}

async fn migration_backfill(store: &Store, db: &sea_orm::DatabaseConnection, schema: &str) {
    // Recreate the prior schema at this owned synthetic database only. No live
    // binary runs concurrently; this tests preservation during an operator upgrade.
    db.execute_unprepared(&format!(
        "DROP TABLE \"{schema}\".execution_watch;
        ALTER TABLE \"{schema}\".reservations DROP COLUMN automatic_recoveries;
        DELETE FROM \"{schema}\".schema_version WHERE version=9;"
    ))
    .await
    .unwrap();
    store.migrate().await.unwrap();
    store.ready().await.unwrap();
}

#[tokio::test]
async fn an_expired_execution_cannot_automatically_return_to_a_disabled_host() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let observed = tokio::spawn(async move {
        let flow = Flow::prepare(&store).await;
        flow.reserve(&store).await;
        store
            .claim(&flow.host, flow.initial.id, Uuid::now_v7(), 120_000)
            .await
            .unwrap();
        expire(&db, &schema, flow.initial.id).await;
        db.execute_unprepared(&format!(
            "UPDATE \"{schema}\".operators SET enabled=false WHERE id='{}'",
            flow.host.id()
        ))
        .await
        .unwrap();
        let candidate = store
            .expired_executions(&flow.supervisor, flow.initial.cohort, 1)
            .await
            .unwrap()[0];
        let recovered = store
            .recover_expired(&flow.supervisor, candidate)
            .await
            .unwrap();
        assert_eq!(recovered.state.phase, Phase::Quarantined);
        assert_eq!(recovered.state.economy, flow.initial.state.economy);
        assert!(recovered.state.history.is_empty());
        assert!(
            store
                .expired_executions(&flow.supervisor, flow.initial.cohort, 1)
                .await
                .unwrap()
                .is_empty()
        );
    })
    .await;
    sandbox.close().await;
    observed.expect("disabled host needs explicit supervision");
}

async fn expire(db: &sea_orm::DatabaseConnection, schema: &str, run: Uuid) {
    db.execute_unprepared(&format!("UPDATE \"{schema}\".runs SET state=convert_to(jsonb_set(convert_from(state,'UTF8')::jsonb,'{{execution,lease_until}}','1'::jsonb)::text,'UTF8') WHERE id='{run}';
        UPDATE \"{schema}\".execution_watch SET lease_until=1 WHERE run='{run}';")).await.unwrap();
}

async fn renewed_hint(
    store: &Store,
    db: &sea_orm::DatabaseConnection,
    schema: &str,
    flow: &Flow,
    claim: &Claimed,
    expired: ExpiredExecution,
) {
    // A stale queue hint can outlive a renewal: restore the live saved deadline,
    // then let the actual renewal transaction update both state and expiry index.
    let until = claim.lease_until;
    let run = flow.initial.id;
    db.execute_unprepared(&format!("UPDATE \"{schema}\".runs SET state=convert_to(jsonb_set(convert_from(state,'UTF8')::jsonb,'{{execution,lease_until}}','{until}'::jsonb)::text,'UTF8') WHERE id='{run}';")).await.unwrap();
    store.renew(&flow.host, claim.lease, 120_000).await.unwrap();
    assert!(
        store
            .recover_expired(&flow.supervisor, expired)
            .await
            .is_err(),
        "a renewed deadline wins over an old discovery hint"
    );
    assert!(
        store
            .expired_executions(&flow.supervisor, flow.initial.cohort, 1)
            .await
            .unwrap()
            .is_empty()
    );
    expire(db, schema, run).await;
}
