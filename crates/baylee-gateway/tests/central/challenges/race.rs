//! A real slow Central claim must survive the delivery worker's crash recovery.
use super::*;
use futures_util::FutureExt;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, TransactionTrait};

async fn binding(db: &DatabaseConnection, attempt: Uuid) -> (Uuid, String) {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT game,touched_at::text AS touched FROM challenge_run WHERE attempt=$1",
            vec![attempt.into()],
        ))
        .await
        .unwrap()
        .expect("the in-flight binding must remain present");
    (
        row.try_get("", "game").unwrap(),
        row.try_get("", "touched").unwrap(),
    )
}

pub(super) fn start_while_recovery_runs(
    runtime: &Runtime,
    operator: &DatabaseConnection,
    gateway: &common::Gateway,
    token: &str,
    path: &str,
    attempt: [u8; 16],
) -> serde_json::Value {
    runtime.block_on(async {
        let attempt = Uuid::from_bytes(attempt);
        let db = sea_orm::Database::connect(gateway.database_url()).await.unwrap();
        let (game, before) = binding(&db, attempt).await;
        // Observe a real delivery tick, then start shortly before its next tick.
        // Central's real five-second SQL deadline remains unchanged.
        tokio::time::timeout(Duration::from_secs(12), async {
            while binding(&db, attempt).await.1 == before {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }).await.expect("delivery rotates the reserved binding");
        tokio::time::sleep(Duration::from_secs(8)).await;
        let hold = operator.begin().await.unwrap();
        hold.query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT id FROM central_identity.accounts WHERE id=(SELECT account_id
             FROM central_identity.challenge_attempts WHERE id=$1) FOR UPDATE",
            vec![attempt.into()],
        )).await.unwrap().expect("account locked before Claim");
        let blocker: i32 = hold.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT pg_backend_pid() AS pid")).await.unwrap().unwrap().try_get("", "pid").unwrap();
        let port = gateway.port;
        let token = token.to_owned();
        let path = path.to_owned();
        let start = tokio::task::spawn_blocking(move || common::http(port, "POST", &path, Some(&token), "{}"));
        let checked = std::panic::AssertUnwindSafe(async {
            tokio::time::timeout(Duration::from_secs(2), async {
                loop {
                    let blocked: i64 = operator.query_one_raw(Statement::from_sql_and_values(
                        DbBackend::Postgres,
                        "SELECT count(*)::bigint AS blocked FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid))",
                        vec![blocker.into()],
                    )).await.unwrap().unwrap().try_get("", "blocked").unwrap();
                    if blocked > 0 { break; }
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
            }).await.expect("Claim really waits on the held account");
            db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                "UPDATE challenge_run SET created_at=clock_timestamp()-interval '180 seconds' WHERE attempt=$1",
                vec![attempt.into()],
            )).await.unwrap();
            let before = binding(&db, attempt).await.1;
            tokio::time::timeout(Duration::from_secs(4), async {
                while binding(&db, attempt).await.1 == before {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            }).await.expect("reconciliation encountered the in-flight start");
            assert_eq!(binding(&db, attempt).await.0, game);
        }).catch_unwind().await;
        // Always release the account and join our caller, including a failed assertion.
        hold.rollback().await.unwrap();
        let started = start.await.unwrap();
        db.close().await.unwrap();
        if let Err(panic) = checked { std::panic::resume_unwind(panic); }
        assert_eq!(started.0, 200, "the same claim starts once the account lock is released");
        let grant: serde_json::Value = serde_json::from_str(&started.1).unwrap();
        assert_eq!(grant["game_id"], game.to_string());
        grant
    })
}
