use super::{Flow, Sandbox, operator, verified_fixture};
use baylee_tower::v2::{package::Build, run::Outcome};
use baylee_tower_api::ranking::{Lane, Listing, Name};
use baylee_tower_store::{Error, Principal, Role, Store};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use uuid::Uuid;

async fn finish(store: &Store, flow: &Flow, verifier: &Principal) {
    let mut next = flow.clone();
    for stage in 1..=8 {
        let saved = verified_fixture(store, &next, verifier, Outcome::Victory).await;
        let private = store
            .ranking(flow.account, flow.initial.cohort, Lane::House)
            .await
            .unwrap();
        assert!(private.leaders.is_empty(), "completion never opts in");
        if stage < 8 {
            assert!(private.best.is_none(), "partial runs are not ranked");
        }
        next = flow.continued(saved);
    }
}

#[tokio::test]
async fn verified_completion_is_private_and_house_and_echo_runs_never_mix() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let observed = tokio::spawn(async move {
        let first = Flow::prepare(&store).await;
        let cohort = first.initial.cohort;
        let unknown = Uuid::now_v7();
        let empty = store.ranking(unknown, cohort, Lane::Echoes).await.unwrap();
        assert_eq!(empty.listing, Listing::default());
        assert!(empty.best.is_none());
        assert!(
            store
                .listing(unknown, 0, true, Some(&name(1)))
                .await
                .is_err()
        );
        let publisher = store.authenticate_operator([1; 32]).await.unwrap();
        for stage in 2..=8 {
            let build = Build::freeze(&first.initial.state, &super::publication().catalog).unwrap();
            for archetype in 1..=3 {
                store
                    .publish_house(&publisher, cohort, stage, archetype, &build)
                    .await
                    .unwrap();
            }
        }
        let verifier = operator(&store, Role::Verifier, 5).await;
        finish(&store, &first, &verifier).await;
        let best = store
            .ranking(first.account, cohort, Lane::House)
            .await
            .unwrap()
            .best
            .unwrap();
        assert_eq!(
            (best.wins, best.defeats, best.rank, best.name),
            (8, 0, None, None)
        );
        assert!(
            store
                .ranking(first.account, cohort, Lane::Echoes)
                .await
                .unwrap()
                .best
                .is_none()
        );
        let second = first.another(&store).await;
        finish(&store, &second, &verifier).await;
        assert!(
            store
                .ranking(second.account, cohort, Lane::House)
                .await
                .unwrap()
                .best
                .is_none()
        );
        assert_eq!(
            store
                .ranking(second.account, cohort, Lane::Echoes)
                .await
                .unwrap()
                .best
                .unwrap()
                .wins,
            8
        );
        for flow in [&first, &second] {
            store
                .listing(flow.account, 0, true, Some(&name(1)))
                .await
                .unwrap();
        }
        assert_eq!(
            store
                .ranking(first.account, cohort, Lane::Echoes)
                .await
                .unwrap()
                .leaders
                .len(),
            1
        );
        assert_eq!(
            store
                .ranking(second.account, cohort, Lane::House)
                .await
                .unwrap()
                .leaders
                .len(),
            1
        );
    })
    .await;
    sandbox.close().await;
    observed.expect("verified completed ranking fixture");
}

fn name(at: u32) -> Name {
    Name {
        display: format!("Player{at}"),
        tag: 4096 + at,
    }
}

#[tokio::test]
async fn ranking_ties_top_bound_privacy_fences_and_deletion_are_consistent() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let observed = tokio::spawn(async move {
        let first = Flow::prepare(&store).await;
        let cohort = first.initial.cohort;
        let mut players = Vec::new();
        // Query fixtures only: the separate test above exercises verified ingestion.
        for at in 0..28 {
            let account = first.another(&store).await.account;
            db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                format!("INSERT INTO \"{schema}\".personal_bests(account,cohort,house,wins,defeats,listed) VALUES($1,$2,false,8,$3,false)"),
                [account.into(), cohort.into(), i16::from(at >= 2).into()])).await.unwrap();
            store.listing(account, 0, true, Some(&name(at))).await.unwrap();
            players.push(account);
        }
        let last = players[27];
        let board = store.ranking(last, cohort, Lane::Echoes).await.unwrap();
        assert!(board.valid());
        assert_eq!(board.leaders.len(), 25);
        assert_eq!((board.leaders[0].rank, board.leaders[1].rank, board.leaders[2].rank), (Some(1), Some(1), Some(3)));
        assert_eq!(board.best.unwrap().rank, Some(3));
        assert!(board.leaders.iter().all(|s| !s.yours), "own best outside bounded podium");
        let hidden = store.listing(last, 1, false, None).await.unwrap();
        assert_eq!(hidden, store.listing(last, 1, false, None).await.unwrap(), "lost ACK retry");
        assert_eq!(store.listing(last, 0, true, Some(&name(27))).await.unwrap_err(), Error::Conflict, "stale publication cannot revive visibility");
        let private = store.ranking(last, cohort, Lane::Echoes).await.unwrap();
        assert_eq!(private.best.unwrap().rank, None);
        let current_name = name(27);
        let (a,b) = tokio::join!(store.listing(last, 2, true, Some(&current_name)), store.listing(last, 2, false, None));
        assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
        let now = store.ranking(last, cohort, Lane::Echoes).await.unwrap().listing;
        store.listing(last, now.revision, true, Some(&name(27))).await.unwrap();
        store.ranking_identity(last, Some(&name(99))).await.unwrap();
        assert_eq!(store.ranking(last, cohort, Lane::Echoes).await.unwrap().best.unwrap().name.unwrap(), name(99));
        store.ranking_identity(last, None).await.unwrap();
        assert!(!store.ranking(last, cohort, Lane::Echoes).await.unwrap().listing.listed);
        db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres, format!("DELETE FROM \"{schema}\".players WHERE account=$1"), [players[0].into()])).await.unwrap();
        let after = store.ranking(players[1], cohort, Lane::Echoes).await.unwrap();
        assert_eq!((after.leaders[0].rank, after.leaders[1].rank), (Some(1), Some(2)));
        assert!(store.ranking(first.account, Uuid::now_v7(), Lane::Echoes).await.is_err());
        assert!(store.ranking(first.account, cohort, Lane::House).await.unwrap().leaders.is_empty());
    }).await;
    sandbox.close().await;
    observed.expect("bounded ranking and publication privacy fixture");
}
