//! Read-only proof of the two independently registered executors behind the visible position.
use super::{DatabaseConnection, Id, Player, Registered, Runtime, Served};
use baylee_account::{
    CentralHttp,
    ranked::{
        self,
        replica::{self, SeatStatus},
    },
    sdk,
};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use uuid::Uuid;

pub(super) fn receipts(rt: &Runtime, db: &DatabaseConnection, id: Id, expected: i64) {
    let query = Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT count(DISTINCT account_id) AS seats FROM central_identity.ranked_receipts WHERE match_id=$1",
        [Uuid::from_bytes(id).into()],
    );
    let row = rt.block_on(db.query_one_raw(query)).unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "seats").unwrap(), expected);
}

pub(super) fn assignments(
    served: &Served,
    players: &[Player; 2],
    pair: [&Registered; 2],
    id: Id,
) -> u64 {
    let http = CentralHttp::new(served.endpoint.clone());
    let [leader, verifier] = pair.map(|host| {
        let request = ranked::HostRequest::Replicate {
            request: replica::HostRequest::Inspect { match_id: id },
        };
        let ranked::HostResponse::Replica {
            response: replica::HostResponse::Assignment { work },
        } = sdk::ranked::host(&http, &host.token, request).unwrap()
        else {
            panic!("approved host should have an authorized assignment")
        };
        assert_eq!(work.fence.match_id, id);
        assert_ne!(work.accounts[0], work.accounts[1]);
        for p in players {
            assert!(!work.accounts.contains(&p.account.public_id()));
        }
        work
    });
    assert_eq!(leader.role, replica::Role::Primary);
    assert_eq!(verifier.role, replica::Role::Standby);
    assert_eq!(leader.seed, verifier.seed);
    assert_eq!(leader.compatibility, verifier.compatibility);
    assert_eq!(leader.decks, verifier.decks);
    assert_eq!(leader.fence, verifier.fence);
    assert!(
        leader
            .accounts
            .iter()
            .all(|a| !verifier.accounts.contains(a))
    );
    leader.seed
}

pub(super) fn quorum(rt: &Runtime, db: &DatabaseConnection, status: &SeatStatus, pair: [Id; 2]) {
    assert_ne!(pair[0], pair[1]);
    let ids = pair.map(Uuid::from_bytes);
    let query = Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT count(DISTINCT host_id)=2 AND count(DISTINCT evidence)=1 AS agreed
         FROM central_identity.ranked_witnesses WHERE match_id=$1 AND generation=$2
          AND position=$3 AND chain=$4 AND host_id IN ($5,$6)",
        [
            Uuid::from_bytes(status.fence.match_id).into(),
            i64::try_from(status.fence.generation).unwrap().into(),
            i64::try_from(status.head.position).unwrap().into(),
            status.head.chain.to_vec().into(),
            ids[0].into(),
            ids[1].into(),
        ],
    );
    let row = rt.block_on(db.query_one_raw(query)).unwrap().unwrap();
    assert!(
        row.try_get::<bool>("", "agreed").unwrap(),
        "separate verified hosts must agree on private rules state for this exact public head"
    );
}
