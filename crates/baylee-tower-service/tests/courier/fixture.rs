//! Synthetic trusted-verifier persistence fixture, not evidence of legal game replay.
#![forbid(unsafe_code)]
use baylee_tower::v2::{
    package::{Build, Seeds},
    run::{Outcome, Settlement},
};
use baylee_tower_store::{CONSENT_REVISION, Entropy, Principal, Reserve, Role, Start, Store};
use central_api::Token;
use uuid::Uuid;

pub async fn operator(store: &Store, role: Role) -> Principal {
    let token = Token::generate().unwrap();
    store
        .provision_operator("synthetic delivery actor", role, token.digest())
        .await
        .unwrap();
    store.authenticate_operator(token.digest()).await.unwrap()
}

pub async fn event(store: &Store, account: Uuid) {
    let (cohort, host) = prepare(store).await;
    let run = store
        .start(
            account,
            Uuid::now_v7(),
            Start {
                cohort,
                starter: 0,
                consent: CONSENT_REVISION,
            },
            [7; 32],
        )
        .await
        .unwrap();
    store
        .reserve(
            account,
            Uuid::now_v7(),
            Reserve {
                run: run.id,
                revision: run.state.revision,
                host: host.id(),
            },
            Entropy {
                opponent: [11; 32],
                game: Seeds {
                    shuffle: 11,
                    policy: 12,
                },
            },
            4096,
        )
        .await
        .unwrap();
    let claimed = store
        .claim(&host, run.id, Uuid::now_v7(), 120_000)
        .await
        .unwrap();
    let ack = store
        .append(
            &host,
            claimed.lease,
            claimed.prefix,
            b"synthetic delivery terminal\n",
        )
        .await
        .unwrap();
    store.seal(&host, claimed.lease, ack.prefix).await.unwrap();
    let verifier = operator(store, Role::Verifier).await;
    let job = store
        .claim_verification(
            &verifier,
            claimed.lease.reservation,
            Uuid::now_v7(),
            120_000,
        )
        .await
        .unwrap();
    store
        .settle_verified(
            &verifier,
            job.lease,
            &Settlement {
                binding: job.lease.binding,
                outcome: Outcome::Victory,
                turns: 42,
                used: [0; 2],
            },
            [77; 32],
        )
        .await
        .unwrap();
}

async fn prepare(store: &Store) -> (Uuid, Principal) {
    let publication = super::support::publication();
    let cohort = store.publish(&publication).await.unwrap();
    let publisher = operator(store, Role::Publisher).await;
    for archetype in 1..=3 {
        store
            .publish_house(
                &publisher,
                cohort,
                1,
                archetype,
                &Build {
                    deck: publication.starters[usize::from(archetype - 1)]
                        .deck
                        .clone(),
                    active: Vec::new(),
                    potions: [0; 2],
                },
            )
            .await
            .unwrap();
    }
    let host = operator(store, Role::Host).await;
    store
        .advertise(
            &host,
            std::slice::from_ref(&publication.cohort.executor),
            std::slice::from_ref(&publication.cohort.executor),
        )
        .await
        .unwrap();
    (cohort, host)
}
