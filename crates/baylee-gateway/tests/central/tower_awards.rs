//! Real Central delivery of synthetic verified facts; no game-rules test is implied.
use super::{Served, fresh, tower_native::support};
use baylee_account::{CentralHttp, Token, sdk, wire::Discovery};
use baylee_tower_service::{Courier, DeliveryStep};
use baylee_tower_store::Role;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use std::{future::pending, time::Duration};
use tokio::runtime::Runtime;
use uuid::Uuid;
#[path = "../../../baylee-tower-service/tests/courier/fixture.rs"]
mod fixture;

pub(super) const ORIGIN: &str = "http://127.0.0.1:29871";
pub(super) fn enabled() -> bool {
    std::env::var("BAYLEE_CENTRAL_TOWER_AWARDS_ONLY").as_deref() == Ok("1")
}

pub(super) fn run(runtime: &Runtime, served: &Served, operator: &DatabaseConnection) {
    let account = fresh::register(runtime, &served.service, &served.endpoint, "courier_player", br#"{"version":1,"gateways":[],"preferences":{},"legacy":null,"chat":{"devices":[],"installations":[]}}"#);
    let http = CentralHttp::new(served.endpoint.clone());
    let Discovery::Central { identity } = sdk::discover(&http, "courier_player".into()).unwrap()
    else {
        panic!("real enrolled Central identity");
    };
    let sandbox = runtime.block_on(support::Sandbox::open());
    let checked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let work = runtime.spawn(deliver(
            sandbox.store.clone(),
            sandbox.db.clone(),
            sandbox.schema.clone(),
            served.endpoint.clone(),
            operator.clone(),
            identity.account,
        ));
        runtime.block_on(work).unwrap();
        let progress = account.tower_award_progress().unwrap();
        assert_eq!(
            (
                progress.duels,
                progress.victories,
                progress.clears,
                progress.flawless
            ),
            (1, 1, 0, 0)
        );
        let export = account.tower_award_export(None).unwrap();
        assert_eq!(
            export.receipts.len(),
            1,
            "lost acknowledgment did not add a second receipt"
        );
        assert_eq!(export.receipts[0].origin, ORIGIN);
        assert_eq!(export.receipts[0].fact.stage, 1);
        assert!(export.next.is_none());
    }));
    runtime.block_on(sandbox.close());
    if let Err(panic) = checked {
        std::panic::resume_unwind(panic);
    }
    println!(
        "Durable Baylee outbox → automatic courier → real Central HTTP → atomic permanent progress → account SDK passed, including lost ACK retry."
    );
}

async fn deliver(
    store: baylee_tower_store::Store,
    db: DatabaseConnection,
    schema: String,
    endpoint: central_api::Endpoint,
    operator: DatabaseConnection,
    account: [u8; 16],
) {
    fixture::event(&store, Uuid::from_bytes(account)).await;
    let issuer = Token::generate().unwrap();
    central_postgres::provision_tower_award_issuer(&operator, ORIGIN, &issuer, true)
        .await
        .unwrap();
    let principal = fixture::operator(&store, Role::Courier).await;
    let mut wrong = Courier::new(
        store.clone(),
        principal.clone(),
        &endpoint,
        ORIGIN,
        Token::generate().unwrap(),
    )
    .unwrap();
    assert_eq!(wrong.next(pending()).await.unwrap(), DeliveryStep::Deferred);
    db.execute_unprepared(&format!(
        "UPDATE \"{schema}\".award_outbox SET retry_after=0"
    ))
    .await
    .unwrap();
    let mut courier = Courier::new(store.clone(), principal, &endpoint, ORIGIN, issuer).unwrap();
    let finished = async {
        loop {
            let row=db.query_one_raw(Statement::from_string(DbBackend::Postgres,format!("SELECT count(*) AS pending FROM \"{schema}\".award_outbox WHERE delivered_at IS NULL"))).await.unwrap().unwrap();
            if row.try_get::<i64>("", "pending").unwrap() == 0 {
                return;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    };
    let summary = tokio::time::timeout(Duration::from_secs(10), courier.serve(finished))
        .await
        .unwrap();
    assert_eq!(
        (summary.recorded, summary.deferred, summary.unavailable),
        (1, 0, 0)
    );
    db.execute_unprepared(&format!(
        "UPDATE \"{schema}\".award_outbox SET delivered_at=NULL,lease_until=1"
    ))
    .await
    .unwrap();
    assert_eq!(
        courier.next(pending()).await.unwrap(),
        DeliveryStep::Recorded
    );
    assert_eq!(courier.next(pending()).await.unwrap(), DeliveryStep::Idle);
}
