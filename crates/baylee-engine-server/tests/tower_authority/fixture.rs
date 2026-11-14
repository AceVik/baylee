use super::support;
use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_tower::{
    cards::{Card, Catalog, Collection, Stack},
    v2::{
        execution::MAX_RECORD_BYTES,
        package::{Build, Cohort, Seeds},
    },
};
use baylee_tower_store::{
    CONSENT_REVISION, Claimed, Entropy, Principal, Publication, Reserve, Role, Start, Store,
    VerificationJob,
};
use uuid::Uuid;

pub(super) struct Flow {
    pub account: Uuid,
    pub run: Uuid,
    pub host: Principal,
    pub verifier: Principal,
    pub job: VerificationJob,
}

impl Flow {
    pub async fn prepare(store: &Store, corrupt: bool) -> Self {
        let publication = publication();
        let cohort = store.publish(&publication).await.unwrap();
        let publisher = operator(store, Role::Publisher, 1).await;
        let host = operator(store, Role::Host, 2).await;
        let verifier = operator(store, Role::Verifier, 3).await;
        store
            .advertise(&host, &[publication.cohort.executor])
            .await
            .unwrap();
        for archetype in 1..=3 {
            store
                .publish_house(
                    &publisher,
                    cohort,
                    1,
                    archetype,
                    &Build {
                        deck: publication.starters[0].deck.clone(),
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
        store
            .reserve(
                account,
                Uuid::now_v7(),
                Reserve {
                    run: initial.id,
                    revision: initial.state.revision,
                    host: host.id(),
                },
                Entropy {
                    opponent: [11; 32],
                    game: Seeds {
                        shuffle: 17,
                        policy: 19,
                    },
                },
                MAX_RECORD_BYTES,
            )
            .await
            .unwrap();
        let claimed = store
            .claim(&host, initial.id, Uuid::now_v7(), 120_000)
            .await
            .unwrap();
        retain(store, &host, &claimed, corrupt).await;
        assert_eq!(
            store
                .verification_candidates(&verifier, cohort, 1)
                .await
                .unwrap(),
            [claimed.lease.reservation]
        );
        let job = store
            .claim_verification(
                &verifier,
                claimed.lease.reservation,
                Uuid::now_v7(),
                120_000,
            )
            .await
            .unwrap();
        Self {
            account,
            run: initial.id,
            host,
            verifier,
            job,
        }
    }
}

async fn retain(store: &Store, host: &Principal, claimed: &Claimed, corrupt: bool) {
    let mut bytes = record(claimed);
    if corrupt {
        bytes[0] ^= 1;
    }
    let mut prefix = claimed.prefix;
    for chunk in bytes.chunks(1024) {
        prefix = store
            .append(host, claimed.lease, prefix, chunk)
            .await
            .unwrap()
            .prefix;
    }
    store.seal(host, claimed.lease, prefix).await.unwrap();
}

fn publication() -> Publication {
    let forest = baylee_cards::decks::by_name("Forest").unwrap();
    let mut cards = vec![Card {
        card: forest,
        price: 0,
        basic_land: true,
    }];
    for name in [
        "Llanowar Elves",
        "Giant Growth",
        "Grizzly Bears",
        "Lightning Bolt",
        "Shock",
        "Counterspell",
        "Opt",
        "Duress",
    ] {
        cards.push(Card {
            card: baylee_cards::decks::by_name(name).unwrap(),
            price: 2,
            basic_land: false,
        });
    }
    let catalog = Catalog { cards };
    let starter = Collection {
        owned: Vec::new(),
        deck: vec![Stack {
            card: forest,
            copies: 40,
        }],
    };
    let mut publication = support::publication();
    publication.cohort = Cohort::new(baylee_engine_server::variant::identity(), &catalog).unwrap();
    publication.catalog = catalog;
    publication.starters = [starter.clone(), starter.clone(), starter];
    publication
}

async fn operator(store: &Store, role: Role, secret: u8) -> Principal {
    store
        .provision_operator("synthetic native verifier", role, [secret; 32])
        .await
        .unwrap();
    store.authenticate_operator([secret; 32]).await.unwrap()
}

fn record(claimed: &Claimed) -> Vec<u8> {
    let identity = baylee_engine_server::variant::identity();
    let preset = claimed
        .package
        .preset(&claimed.catalog, &identity, |card| {
            Some(baylee_cards::decks::reference_print(card))
        })
        .unwrap();
    let mut live =
        baylee_engine_server::variant::HostedSession::recorded_for_execution(&preset, &identity)
            .unwrap();
    let me = PlayerId::new(0);
    for _ in 0..32 {
        if live.needs_pump() {
            live.pump_slice(1);
        } else if matches!(live.pending(), Pending::Priority { player, .. } if *player == me) {
            break;
        } else {
            live.act_slice(me, live.house_action(me).unwrap(), 0)
                .unwrap();
        }
    }
    live.act_slice(me, PlayerAction::Concede, 0).unwrap();
    live.take_record()
}
