use super::{Flow, Sandbox};
use baylee_tower::{Refusal, v2::execution::Prefix};
use baylee_tower_store::{Error, MAX_ACCOUNT_RECORD_BYTES, MAX_ACCOUNT_RECORD_CHUNKS, Store};
use sea_orm::ConnectionTrait;
use uuid::Uuid;

#[tokio::test]
async fn lost_acks_and_failed_writes_never_duplicate_or_partially_commit_records() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let observed = tokio::spawn(async move {
        let flow = Flow::prepare(&store).await;
        flow.reserve(&store).await;
        let claimed = store.claim(&flow.host, flow.initial.id, Uuid::now_v7(), 120_000).await.unwrap();
        let mut races = Vec::new();
        for _ in 0..8 {
            let store = store.clone();
            let host = flow.host.clone();
            let lease = claimed.lease;
            races.push(tokio::spawn(async move { store.append(&host, lease, Prefix::default(), b"synthetic header\n").await }));
        }
        let mut responses = Vec::new();
        for race in races { responses.push(race.await.unwrap().unwrap()); }
        assert!(responses.iter().all(|response| response == &responses[0]));
        let head = responses[0].prefix;
        assert_eq!(head.chunks, 1);
        assert_eq!(head.bytes, 17);
        let reopened = Store::new(db.clone(), &schema).unwrap();
        let read = reopened.read_chunk(&flow.host, claimed.lease, 0).await.unwrap();
        assert_eq!(read.data, b"synthetic header\n");
        assert_eq!(read.previous, Prefix::default());
        assert_eq!(read.head, head);
        assert_eq!(reopened.append(&flow.host, claimed.lease, Prefix::default(), b"changed\n").await, Err(Error::Conflict));
        let before = store.get(flow.account, flow.initial.id).await.unwrap();
        db.execute_unprepared(&format!("ALTER TABLE \"{schema}\".record_chunks ADD CONSTRAINT injected_failure CHECK (seq=0)")).await.unwrap();
        assert_eq!(store.append(&flow.host, claimed.lease, head, b"next\n").await, Err(Error::Unavailable));
        assert_eq!(store.get(flow.account, flow.initial.id).await.unwrap(), before);
        db.execute_unprepared(&format!("ALTER TABLE \"{schema}\".record_chunks DROP CONSTRAINT injected_failure")).await.unwrap();
        let row = db.query_one_raw(sea_orm::Statement::from_string(sea_orm::DbBackend::Postgres,
            format!("SELECT record_bytes=17 AND record_chunks=1 AS quota_ok FROM \"{schema}\".players"))).await.unwrap().unwrap();
        assert!(row.try_get::<bool>("", "quota_ok").unwrap());
        for (field, limit) in [("record_bytes", MAX_ACCOUNT_RECORD_BYTES), ("record_chunks", MAX_ACCOUNT_RECORD_CHUNKS)] {
            db.execute_unprepared(&format!("UPDATE \"{schema}\".players SET {field}={limit}")).await.unwrap();
            assert_eq!(store.append(&flow.host, claimed.lease, head, b"next\n").await, Err(Error::Refused(Refusal::Capacity)));
            assert_eq!(store.get(flow.account, flow.initial.id).await.unwrap(), before);
            assert_eq!(store.append(&flow.host, claimed.lease, Prefix::default(), b"synthetic header\n").await.unwrap(), responses[0]);
            db.execute_unprepared(&format!("UPDATE \"{schema}\".players SET record_bytes=17,record_chunks=1")).await.unwrap();
        }
        let second = store.append(&flow.host, claimed.lease, head, b"next\n").await.unwrap();
        assert_eq!(second.prefix.chunks, 2);
        assert_eq!(second.prefix.bytes, 22);
        assert!(store.read_chunk(&flow.next_host, claimed.lease, 0).await.is_err());
        assert!(store.read_chunk(&flow.host, claimed.lease, 2).await.is_err());
    }).await;
    sandbox.close().await;
    observed.expect("durability scenario completed");
}

#[tokio::test]
async fn corrupt_bytes_and_reservation_capacity_are_not_acknowledged_as_new_progress() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let observed = tokio::spawn(async move {
        let flow = Flow::prepare(&store).await;
        flow.reserve(&store).await;
        let claimed = store
            .claim(&flow.host, flow.initial.id, Uuid::now_v7(), 120_000)
            .await
            .unwrap();
        let bytes = vec![b'x'; 4096];
        let ack = store
            .append(&flow.host, claimed.lease, Prefix::default(), &bytes)
            .await
            .unwrap();
        assert_eq!(
            store
                .append(&flow.host, claimed.lease, ack.prefix, b"x")
                .await,
            Err(Error::Refused(Refusal::Capacity))
        );
        let before = store.get(flow.account, flow.initial.id).await.unwrap();
        db.execute_unprepared(&format!(
            "UPDATE \"{schema}\".record_chunks SET data=convert_to('corrupt','UTF8')"
        ))
        .await
        .unwrap();
        assert!(
            store
                .read_chunk(&flow.host, claimed.lease, 0)
                .await
                .is_err()
        );
        assert_eq!(
            store
                .append(&flow.host, claimed.lease, Prefix::default(), b"corrupt")
                .await,
            Err(Error::Unavailable)
        );
        assert_eq!(
            store.get(flow.account, flow.initial.id).await.unwrap(),
            before
        );
        db.execute_unprepared(&format!(
            "UPDATE \"{schema}\".record_chunks SET data=convert_to(repeat('x',4096),'UTF8')"
        ))
        .await
        .unwrap();
        assert_eq!(
            store
                .append(&flow.host, claimed.lease, Prefix::default(), &bytes)
                .await
                .unwrap(),
            ack
        );
    })
    .await;
    sandbox.close().await;
    observed.expect("bounded record/corruption scenario completed");
}
