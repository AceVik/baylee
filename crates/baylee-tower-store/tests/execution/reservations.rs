use super::{Flow, Sandbox, entropy};
use baylee_tower::v2::{
    package::Origin,
    run::{Command, Phase},
};
use baylee_tower_store::{Error, Store};
use uuid::Uuid;

#[tokio::test]
async fn reservation_locks_one_exact_package_before_any_claim() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let observed = tokio::spawn(async move {
        let flow = Flow::prepare(&store).await;
        rejected_host_and_owner(&store, &flow).await;
        let request = Uuid::now_v7();
        let mut races = Vec::new();
        for seed in 1..=8 {
            let store = store.clone();
            let choice = flow.choice();
            let account = flow.account;
            races.push(tokio::spawn(async move {
                store
                    .reserve(account, request, choice, entropy(seed), 4096)
                    .await
            }));
        }
        let mut responses = Vec::new();
        for race in races {
            responses.push(race.await.unwrap().unwrap());
        }
        assert!(responses.iter().all(|response| response == &responses[0]));
        assert_eq!(responses[0].state.phase, Phase::Reserved);
        assert_eq!(responses[0].state.economy, flow.initial.state.economy);
        assert_eq!(responses[0].state.revision, 1);
        claimed_and_repeated(&store, &flow, request, &responses[0]).await;
    })
    .await;
    sandbox.close().await;
    observed.expect("reservation scenario completed");
}

async fn rejected_host_and_owner(store: &Store, flow: &Flow) {
    assert_eq!(
        store
            .reserve(
                Uuid::now_v7(),
                Uuid::now_v7(),
                flow.choice(),
                entropy(1),
                4096
            )
            .await,
        Err(Error::NotFound)
    );
    // A declared artifact skew cannot be silently replaced by another engine.
    let mut skew = super::publication().cohort.executor;
    skew.build.push('x');
    store.advertise(&flow.host, &[skew]).await.unwrap();
    assert!(
        store
            .reserve(
                flow.account,
                Uuid::now_v7(),
                flow.choice(),
                entropy(1),
                4096
            )
            .await
            .is_err()
    );
    assert_eq!(
        store.get(flow.account, flow.initial.id).await.unwrap(),
        flow.initial
    );
    store
        .advertise(&flow.host, &[super::publication().cohort.executor])
        .await
        .unwrap();
}

async fn claimed_and_repeated(
    store: &Store,
    flow: &Flow,
    request: Uuid,
    response: &baylee_tower_store::Snapshot,
) {
    assert!(
        store
            .command(
                flow.account,
                Uuid::now_v7(),
                flow.initial.id,
                1,
                &Command::Refresh {}
            )
            .await
            .is_err()
    );
    assert!(
        store
            .claim(&flow.next_host, flow.initial.id, Uuid::now_v7(), 120_000)
            .await
            .is_err()
    );
    let claim_id = Uuid::now_v7();
    let claimed = store
        .claim(&flow.host, flow.initial.id, claim_id, 120_000)
        .await
        .unwrap();
    assert!(matches!(claimed.package.opponent, Origin::House { .. }));
    assert_eq!(claimed.package.stage, 1);
    let retry = store
        .claim(&flow.host, flow.initial.id, claim_id, 120_000)
        .await
        .unwrap();
    assert_eq!(claimed.lease, retry.lease);
    assert!(claimed.package == retry.package);
    assert_eq!(
        store
            .reserve(flow.account, request, flow.choice(), entropy(99), 1)
            .await
            .unwrap(),
        *response
    );
    assert!(
        store
            .claim(&flow.host, flow.initial.id, Uuid::now_v7(), 120_000)
            .await
            .is_err()
    );
    let mut changed = flow.choice();
    changed.host = flow.next_host.id();
    assert_eq!(
        store
            .reserve(flow.account, request, changed, entropy(1), 4096)
            .await,
        Err(Error::Conflict)
    );
}
