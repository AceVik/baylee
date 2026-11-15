use super::{Flow, Sandbox, publication};
use baylee_tower_api::Host;
use baylee_tower_store::Store;
use sea_orm::ConnectionTrait;
use uuid::Uuid;

async fn listed(store: &Store, flow: &Flow) -> Vec<Host> {
    store
        .hosts(flow.account, flow.initial.id, None, 8)
        .await
        .unwrap()
}

#[tokio::test]
async fn discovery_requires_approval_exact_free_capacity_and_fresh_heartbeat() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let observed = tokio::spawn(async move {
        let flow = Flow::prepare(&store).await;
        assert!(listed(&store, &flow).await.is_empty(), "no unapproved destination");
        for (host, origin) in [
            (&flow.host, "https://first.example.invalid"),
            (&flow.next_host, "https://second.example.invalid"),
        ] {
            store.approve_host(host.id(), origin, false).await.unwrap();
        }
        let all = listed(&store, &flow).await;
        assert_eq!(all.len(), 2);
        assert!(all[0].id > all[1].id);
        let page = store.hosts(flow.account, flow.initial.id, None, 1).await.unwrap();
        assert_eq!(page, all[..1]);
        assert_eq!(store.hosts(flow.account, flow.initial.id, Some(page[0].id), 1).await.unwrap(), all[1..]);
        assert!(store.hosts(Uuid::now_v7(), flow.initial.id, None, 8).await.is_err());
        let foreign = flow.another(&store).await;
        assert!(store.hosts(foreign.account, flow.initial.id, None, 8).await.is_err());
        assert!(store.hosts(flow.account, flow.initial.id, Some(Uuid::nil()), 8).await.is_err());
        assert!(store.hosts(flow.account, flow.initial.id, None, 17).await.is_err());
        busy_and_skew(&store, &flow).await;
        db.execute_unprepared(&format!(
            "UPDATE \"{schema}\".host_availability SET expires_at=pg_catalog.clock_timestamp()-interval '1 second'"
        )).await.unwrap();
        assert!(listed(&store, &flow).await.is_empty(), "expiry needs no cleanup job");
        let identity = publication().cohort.executor;
        store.advertise(&flow.host, std::slice::from_ref(&identity), std::slice::from_ref(&identity)).await.unwrap();
        assert_eq!(listed(&store, &flow).await.len(), 1);
        db.execute_unprepared(&format!(
            "UPDATE \"{schema}\".operators SET enabled=false WHERE id='{}'", flow.host.id()
        )).await.unwrap();
        assert!(listed(&store, &flow).await.is_empty(), "revocation is immediate");
    }).await;
    sandbox.close().await;
    observed.expect("global host discovery completed");
}

async fn busy_and_skew(store: &Store, flow: &Flow) {
    let identity = publication().cohort.executor;
    let installed = std::slice::from_ref(&identity);
    store.advertise(&flow.host, installed, &[]).await.unwrap();
    let hosts = listed(store, flow).await;
    assert_eq!(hosts.len(), 1);
    assert_eq!(hosts[0].id, flow.next_host.id());
    assert!(
        store
            .host_supports(flow.host.id(), &identity)
            .await
            .unwrap(),
        "busy does not revoke recovery compatibility"
    );
    let mut skew = identity.clone();
    skew.build.push('x');
    assert!(
        store
            .advertise(&flow.next_host, installed, std::slice::from_ref(&skew))
            .await
            .is_err()
    );
    assert!(
        store
            .advertise(
                &flow.next_host,
                installed,
                &[identity.clone(), identity.clone()]
            )
            .await
            .is_err()
    );
    assert!(
        store
            .advertise(&flow.supervisor, installed, installed)
            .await
            .is_err()
    );
    assert_eq!(
        listed(store, flow).await,
        hosts,
        "invalid advertisements are atomic"
    );
    store
        .advertise(
            &flow.next_host,
            std::slice::from_ref(&skew),
            std::slice::from_ref(&skew),
        )
        .await
        .unwrap();
    assert!(
        listed(store, flow).await.is_empty(),
        "another build cannot substitute"
    );
    store.advertise(&flow.next_host, &[], &[]).await.unwrap();
    assert!(
        !store
            .host_supports(flow.next_host.id(), &skew)
            .await
            .unwrap()
    );
    store
        .advertise(&flow.host, installed, installed)
        .await
        .unwrap();
}
