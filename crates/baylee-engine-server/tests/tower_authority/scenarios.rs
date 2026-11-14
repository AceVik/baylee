use super::*;
use sea_orm::{ConnectionTrait as _, TransactionTrait as _};
use tokio::sync::oneshot;

#[tokio::test]
async fn shutdown_and_dropped_request_reap_before_releasing_capacity() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let observed = tokio::spawn(async move {
        let service = verifier().await;
        let flow = Flow::prepare(&store, false).await;
        let before = store.get(flow.account, flow.run).await.unwrap();
        for drop_request in [false, true] {
            // Hold the first record read behind a known owned transaction. This
            // proves cancellation during active work, rather than lucky preflight.
            let lock = db.begin().await.unwrap();
            lock.execute_unprepared(&format!(
                "SELECT account FROM \"{schema}\".players WHERE account='{}' FOR UPDATE",
                flow.account
            ))
            .await
            .unwrap();
            let (stop, stopped) = oneshot::channel();
            let worker = {
                let service = service.clone();
                let store = store.clone();
                let principal = flow.verifier.clone();
                let job = flow.job.clone();
                tokio::spawn(async move {
                    service
                        .adjudicate(&store, &principal, job, async {
                            let _ = stopped.await;
                        })
                        .await
                })
            };
            wait_running(&service, 1).await;
            wait_record_read(&db, &schema).await;
            assert_eq!(
                service
                    .adjudicate(&store, &flow.verifier, flow.job.clone(), pending())
                    .await,
                Err(Error::Busy)
            );
            if drop_request {
                worker.abort();
                assert!(worker.await.unwrap_err().is_cancelled());
            } else {
                stop.send(()).unwrap();
                assert_eq!(worker.await.unwrap(), Err(Error::Cancelled));
            }
            wait_running(&service, 0).await;
            lock.rollback().await.unwrap();
            assert_eq!(store.get(flow.account, flow.run).await.unwrap(), before);
        }
        // The same retained job remains usable: cancellation supplies no outcome.
        assert!(
            service
                .adjudicate(&store, &flow.verifier, flow.job, pending())
                .await
                .is_ok()
        );
    })
    .await;
    sandbox.close().await;
    observed.expect("owned native children were reaped on both cancellation paths");
}

async fn wait_record_read(db: &sea_orm::DatabaseConnection, schema: &str) {
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let row = db.query_one_raw(sea_orm::Statement::from_sql_and_values(
                sea_orm::DbBackend::Postgres,
                "SELECT count(*)::bigint AS n FROM pg_stat_activity WHERE wait_event_type='Lock' AND query LIKE $1",
                vec![format!("%{schema}%").into()],
            )).await.unwrap().unwrap();
            if row.try_get::<i64>("", "n").unwrap() > 0 { return; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.expect("owned worker entered its bounded retained-record read");
}
