use super::{Flow, Sandbox, entropy};
use baylee_tower::{
    Refusal,
    v2::execution::{MAX_CHUNK_BYTES, MAX_RECORD_BYTES, Prefix},
};
use baylee_tower_store::Error;
use sea_orm::ConnectionTrait as _;
use uuid::Uuid;

#[tokio::test]
async fn complete_operation_commits_once_and_mid_batch_failure_leaves_no_prefix() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let observed = tokio::spawn(async move {
        let flow = Flow::prepare(&store).await;
        store.reserve(flow.account, Uuid::now_v7(), flow.choice(), entropy(1), MAX_RECORD_BYTES).await.unwrap();
        let claimed = store.claim(&flow.host, flow.initial.id, Uuid::now_v7(), 120_000).await.unwrap();
        let chunks = vec![vec![b'x'; MAX_CHUNK_BYTES], b"end\n".to_vec()];
        let before = store.get(flow.account, flow.initial.id).await.unwrap();
        db.execute_unprepared(&format!("ALTER TABLE \"{schema}\".record_chunks ADD CONSTRAINT injected_failure CHECK(seq=0)")).await.unwrap();
        assert_eq!(store.append_batch(&flow.host, claimed.lease, claimed.prefix, &chunks).await, Err(Error::Unavailable));
        assert_eq!(store.get(flow.account, flow.initial.id).await.unwrap(), before);
        assert_eq!(store.read_chunk(&flow.host, claimed.lease, 0).await.err(), Some(Error::NotFound));
        let row = db.query_one_raw(sea_orm::Statement::from_string(sea_orm::DbBackend::Postgres,
            format!("SELECT record_bytes=0 AND record_chunks=0 AND NOT EXISTS(SELECT 1 FROM \"{schema}\".record_batches) AS empty FROM \"{schema}\".players"))).await.unwrap().unwrap();
        assert!(row.try_get::<bool>("", "empty").unwrap());
        db.execute_unprepared(&format!("ALTER TABLE \"{schema}\".record_chunks DROP CONSTRAINT injected_failure")).await.unwrap();
        let mut tasks = Vec::new();
        for _ in 0..4 {
            let store=store.clone(); let host=flow.host.clone(); let chunks=chunks.clone(); let lease=claimed.lease;
            tasks.push(tokio::spawn(async move { store.append_batch(&host, lease, Prefix::default(), &chunks).await }));
        }
        let mut receipts = Vec::new();
        for task in tasks { receipts.push(task.await.unwrap().unwrap()); }
        assert!(receipts.iter().all(|receipt| receipt == &receipts[0]));
        assert_eq!(receipts[0].prefix.chunks, 2);
        assert_eq!(receipts[0].prefix.bytes, (MAX_CHUNK_BYTES + 4) as u64);
        let current = store.get(flow.account, flow.initial.id).await.unwrap();
        assert_eq!(store.append_batch(&flow.host, claimed.lease, Prefix::default(), &[vec![b'x'; MAX_CHUNK_BYTES]]).await, Err(Error::Refused(Refusal::Capacity)));
        assert_eq!(store.append_batch(&flow.host, claimed.lease, Prefix::default(), &[b"changed\n".to_vec()]).await, Err(Error::Conflict));
        let extra = store.append_batch(&flow.host, claimed.lease, receipts[0].prefix, &[b"later\n".to_vec()]).await.unwrap();
        assert_eq!(store.append_batch(&flow.host, claimed.lease, Prefix::default(), &chunks).await.unwrap(), receipts[0]);
        assert_eq!(store.get(flow.account, flow.initial.id).await.unwrap().state.execution.unwrap().prefix, extra.prefix);
        assert_ne!(current.state.execution.unwrap().prefix, extra.prefix);
        let row = db.query_one_raw(sea_orm::Statement::from_string(sea_orm::DbBackend::Postgres,
            format!("SELECT record_bytes=(SELECT sum(octet_length(data)) FROM \"{schema}\".record_chunks) AND record_chunks=3 AND (SELECT count(*) FROM \"{schema}\".record_batches)=2 AS exact FROM \"{schema}\".players"))).await.unwrap().unwrap();
        assert!(row.try_get::<bool>("", "exact").unwrap());
    }).await;
    sandbox.close().await;
    observed.expect("atomic complete-operation evidence held");
}
