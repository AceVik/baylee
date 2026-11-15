//! Synthetic byte streams test storage, never claim to be legal game records.
#![forbid(unsafe_code)]
mod batches;
mod discovery;
mod expiry;
mod location;
mod pool;
mod records;
mod recovery;
mod reservations;
#[path = "../support/mod.rs"]
mod support;
mod verification;

use baylee_tower::v2::package::{Build, Seeds};
use baylee_tower_store::{
    CONSENT_REVISION, Entropy, Principal, Reserve, Role, Snapshot, Start, Store,
};
use support::{Sandbox, publication};
use uuid::Uuid;

#[derive(Clone)]
struct Flow {
    account: Uuid,
    initial: Snapshot,
    host: Principal,
    next_host: Principal,
    supervisor: Principal,
}

impl Flow {
    async fn another(&self, store: &Store) -> Self {
        let account = Uuid::now_v7();
        let initial = store
            .start(
                account,
                Uuid::now_v7(),
                Start {
                    cohort: self.initial.cohort,
                    starter: 0,
                    consent: CONSENT_REVISION,
                },
                [19; 32],
            )
            .await
            .unwrap();
        Self {
            account,
            initial,
            ..self.clone()
        }
    }

    fn continued(&self, initial: Snapshot) -> Self {
        Self {
            initial,
            ..self.clone()
        }
    }
    async fn prepare(store: &Store) -> Self {
        let published = publication();
        let cohort = store.publish(&published).await.unwrap();
        let publisher = operator(store, Role::Publisher, 1).await;
        let host = operator(store, Role::Host, 2).await;
        let next_host = operator(store, Role::Host, 3).await;
        let supervisor = operator(store, Role::Supervisor, 4).await;
        for actor in [&host, &next_host] {
            store
                .advertise(
                    actor,
                    std::slice::from_ref(&published.cohort.executor),
                    std::slice::from_ref(&published.cohort.executor),
                )
                .await
                .unwrap();
        }
        for archetype in 1..=3 {
            store
                .publish_house(
                    &publisher,
                    cohort,
                    1,
                    archetype,
                    &Build {
                        deck: published.starters[usize::from(archetype - 1)].deck.clone(),
                        active: Vec::new(),
                        potions: [0; 2],
                    },
                )
                .await
                .unwrap();
        }
        let account = Uuid::now_v7();
        let initial = store
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
        Self {
            account,
            initial,
            host,
            next_host,
            supervisor,
        }
    }

    fn choice(&self) -> Reserve {
        Reserve {
            run: self.initial.id,
            revision: self.initial.state.revision,
            host: self.host.id(),
        }
    }

    async fn reserve(&self, store: &Store) -> Snapshot {
        store
            .reserve(
                self.account,
                Uuid::now_v7(),
                self.choice(),
                entropy(11),
                4096,
            )
            .await
            .unwrap()
    }
}

async fn operator(store: &Store, role: Role, secret: u8) -> Principal {
    store
        .provision_operator("synthetic execution actor", role, [secret; 32])
        .await
        .unwrap();
    store.authenticate_operator([secret; 32]).await.unwrap()
}

fn entropy(seed: u8) -> Entropy {
    Entropy {
        opponent: [seed; 32],
        game: Seeds {
            shuffle: u64::from(seed),
            policy: u64::from(seed) + 1,
        },
    }
}

async fn terminal(store: &Store, flow: &Flow) -> baylee_tower_store::Claimed {
    flow.reserve(store).await;
    let claimed = store
        .claim(&flow.host, flow.initial.id, Uuid::now_v7(), 120_000)
        .await
        .unwrap();
    let ack = store
        .append(
            &flow.host,
            claimed.lease,
            claimed.prefix,
            b"synthetic terminal fixture\n",
        )
        .await
        .unwrap();
    store
        .seal(&flow.host, claimed.lease, ack.prefix)
        .await
        .unwrap();
    claimed
}

async fn verified_fixture(
    store: &Store,
    flow: &Flow,
    verifier: &Principal,
    outcome: baylee_tower::v2::run::Outcome,
) -> Snapshot {
    let claimed = terminal(store, flow).await;
    settle_fixture(store, &claimed, verifier, outcome).await
}

async fn settle_fixture(
    store: &Store,
    claimed: &baylee_tower_store::Claimed,
    verifier: &Principal,
    outcome: baylee_tower::v2::run::Outcome,
) -> Snapshot {
    let job = store
        .claim_verification(verifier, claimed.lease.reservation, Uuid::now_v7(), 120_000)
        .await
        .unwrap();
    let report = baylee_tower::v2::run::Settlement {
        binding: job.lease.binding,
        outcome,
        turns: 42,
        used: [0; 2],
    };
    // This is a trusted-role persistence fixture, NOT an actual replay verifier.
    store
        .settle_verified(verifier, job.lease, &report, [77; 32])
        .await
        .unwrap()
}
