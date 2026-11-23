//! Named destinations and their actual current terms are approved before any deck disclosure.
use super::{CentralHttp, Id, Player, Registered, Served, sdk};
use baylee_account::ranked::{self, replica};
use baylee_account::{
    RankedHostDocuments,
    ranked::replica::{ConsentState, Role, SeatRequest, SeatResponse},
    terms::TermsDocument,
};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use uuid::Uuid;

pub(super) fn unopened(rt: &Runtime, db: &DatabaseConnection, id: Id) {
    let query=Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT (NOT claimed_a AND NOT claimed_b AND deck_a IS NULL AND deck_b IS NULL
           AND NOT EXISTS (SELECT 1 FROM central_identity.ranked_replicas r WHERE r.match_id=m.id
             AND (setup IS NOT NULL OR initial_pools IS NOT NULL))
           AND NOT EXISTS (SELECT 1 FROM central_identity.ranked_commands c WHERE c.match_id=m.id)) AS private
         FROM central_identity.ranked_matches m WHERE m.id=$1", [Uuid::from_bytes(id).into()]);
    let row = rt.block_on(db.query_one_raw(query)).unwrap().unwrap();
    assert!(
        row.try_get::<bool>("", "private").unwrap(),
        "no deck material or claim before both decisions"
    );
}

pub(super) fn no_private_work(served: &Served, host: &Registered, player: &Player, id: Id) {
    let http = CentralHttp::new(served.endpoint.clone());
    let ask = |request| {
        sdk::ranked::host(
            &http,
            &host.token,
            ranked::HostRequest::Replicate { request },
        )
    };
    let ranked::HostResponse::Replica {
        response: replica::HostResponse::Work { items, .. },
    } = ask(replica::HostRequest::Work).unwrap()
    else {
        panic!("expected a bounded host work list")
    };
    assert!(items.iter().all(|w| w.fence.match_id != id));
    let SeatResponse::Status { status } = player
        .account
        .replica(SeatRequest::Status { match_id: id })
        .unwrap()
    else {
        panic!("expected protected status")
    };
    assert!(
        ask(replica::HostRequest::Read {
            fence: status.fence,
            after: 0
        })
        .is_err(),
        "unapproved recipient cannot replay the private opening"
    );
}

pub(super) fn wait_for_terms(http: &CentralHttp, origin: &str) -> TermsDocument {
    let until = Instant::now() + Duration::from_secs(35);
    loop {
        if let Some(doc) = sdk::gateway_terms(http, origin).unwrap() {
            assert!(!doc.texts.is_empty());
            return doc;
        }
        assert!(
            Instant::now() < until,
            "verified host did not publish its actual current terms"
        );
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn offers(player: &Player, match_id: Id, expected: &[&Registered]) -> RankedHostDocuments {
    let until = Instant::now() + Duration::from_secs(90);
    loop {
        let documents = player.account.ranked_host_documents(match_id).unwrap();
        let hosts = &documents.hosts;
        if hosts.len() == expected.len()
            && expected.iter().all(|h| {
                hosts
                    .iter()
                    .any(|o| o.offer.host.host_id == h.id && o.offer.host.origin == h.origin)
            })
        {
            assert_eq!(documents.fence.match_id, match_id);
            assert_eq!(
                hosts
                    .iter()
                    .filter(|o| o.offer.role == Role::Primary)
                    .count(),
                1
            );
            assert_eq!(
                hosts
                    .iter()
                    .filter(|o| o.offer.role == Role::Standby)
                    .count(),
                expected.len() - 1
            );
            for h in hosts {
                let doc = h
                    .document
                    .as_ref()
                    .expect("this host publishes actual terms");
                assert!(!doc.texts.is_empty());
                assert_eq!(h.offer.host.terms.as_ref(), Some(&doc.version));
            }
            return documents;
        }
        assert!(
            Instant::now() < until,
            "current recipient list did not reflect verified operators"
        );
        std::thread::sleep(Duration::from_millis(300));
    }
}

fn status(player: &Player, match_id: Id) -> replica::SeatStatus {
    let SeatResponse::Status { status } = player
        .account
        .replica(SeatRequest::Status { match_id })
        .unwrap()
    else {
        panic!("expected private consent standing");
    };
    status
}

fn withheld_deck(p: &Player, id: Id, expected: ConsentState) {
    let route = status(p, id);
    assert_eq!(route.consent, expected);
    let attempted =
        p.account
            .play_ranked_replica(p.key, &route, Some((p.commitment, &p.deck)), &mut None);
    assert!(
        matches!(
            attempted,
            Ok(None) | Err(baylee_account::Error::TermsRequired)
        ),
        "no seat or deck transfer while either participant still needs to consent"
    );
}

pub(super) fn approve(
    players: &[Player; 2],
    match_id: Id,
    expected: &[&Registered],
    withheld: impl Fn(&[Player; 2]),
) {
    let documents = players.each_ref().map(|p| offers(p, match_id, expected));
    for p in players {
        withheld_deck(p, match_id, ConsentState::Needed);
    }
    withheld(players);
    documents[0].accept(&players[0].account).unwrap();
    withheld_deck(&players[0], match_id, ConsentState::Waiting);
    withheld_deck(&players[1], match_id, ConsentState::Needed);
    withheld(players);
    documents[1].accept(&players[1].account).unwrap();
    for p in players {
        assert_eq!(status(p, match_id).consent, ConsentState::Ready);
    }
}
