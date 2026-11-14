//! Synthetic byte streams test storage, never claim to be legal game records.
#![forbid(unsafe_code)]
mod records;
mod recovery;
mod reservations;
#[path = "../support/mod.rs"]
mod support;

use baylee_tower::v2::package::{Build, Seeds};
use baylee_tower_store::{
    CONSENT_REVISION, Entropy, Principal, Reserve, Role, Snapshot, Start, Store,
};
use support::{Sandbox, publication};
use uuid::Uuid;

struct Flow {
    account: Uuid,
    initial: Snapshot,
    host: Principal,
    next_host: Principal,
    supervisor: Principal,
}

impl Flow {
    async fn prepare(store: &Store) -> Self {
        let published = publication();
        let cohort = store.publish(&published).await.unwrap();
        let publisher = operator(store, Role::Publisher, 1).await;
        let host = operator(store, Role::Host, 2).await;
        let next_host = operator(store, Role::Host, 3).await;
        let supervisor = operator(store, Role::Supervisor, 4).await;
        for actor in [&host, &next_host] {
            store
                .advertise(actor, std::slice::from_ref(&published.cohort.executor))
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
