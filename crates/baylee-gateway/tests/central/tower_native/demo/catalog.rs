//! Small, explicit playable fixtures. These are balance hypotheses, not a release catalog.
use super::super::{Identity, Principal, Store, Uuid, support};
use baylee_tower::{
    cards::{Card, Catalog, Collection, Stack},
    v2::{
        catalog::{Definition, Family, Rarity},
        package::{Build, Cohort},
    },
};
use baylee_tower_store::Publication;

pub(in super::super) fn publication(identity: &Identity) -> Publication {
    if !super::enabled() {
        let mut result = support::publication();
        let forest = card("Forest");
        result.catalog.cards.truncate(1);
        result.catalog.cards[0].card = forest;
        for starter in &mut result.starters {
            starter.deck[0].card = forest;
        }
        result.cohort = Cohort::new(identity.clone(), &result.catalog).unwrap();
        return result;
    }
    playable(identity)
}

fn playable(identity: &Identity) -> Publication {
    let mut catalog = Catalog {
        cards: ["Plains", "Island", "Swamp", "Mountain", "Forest"]
            .map(|name| Card {
                card: card(name),
                price: 0,
                basic_land: true,
                run: None,
            })
            .into(),
    };
    let starters = [
        starter(
            "Forest",
            &[
                ("Llanowar Elves", 4, 4),
                ("Grizzly Bears", 4, 2),
                ("Centaur Courser", 4, 4),
                ("Giant Spider", 3, 4),
                ("Giant Growth", 4, 3),
                ("Elvish Visionary", 4, 4),
            ],
            &mut catalog,
        ),
        starter(
            "Mountain",
            &[
                ("Raging Goblin", 4, 2),
                ("Goblin Piker", 4, 2),
                ("Hill Giant", 4, 3),
                ("Shock", 4, 4),
                ("Lightning Strike", 4, 5),
                ("Goblin Arsonist", 3, 2),
            ],
            &mut catalog,
        ),
        starter(
            "Plains",
            &[
                ("Savannah Lions", 4, 3),
                ("Silvercoat Lion", 4, 2),
                ("Pacifism", 4, 4),
                ("Raise the Alarm", 4, 3),
                ("Benalish Hero", 4, 2),
                ("Serra Angel", 3, 6),
            ],
            &mut catalog,
        ),
    ];
    catalog.cards.sort_by_key(|entry| entry.card);
    for starter in &starters {
        starter.validate(&catalog).unwrap();
    }
    Publication {
        cohort: Cohort::new(identity.clone(), &catalog).unwrap(),
        catalog,
        starters: starters.into(),
    }
}

fn card(name: &str) -> baylee_core::ids::CardIndex {
    let id =
        baylee_cards::decks::by_name(name).unwrap_or_else(|| panic!("missing demo card: {name}"));
    assert_eq!(
        baylee_cards::by_index(id).unwrap().coverage,
        baylee_cards::dsl::Coverage::Implemented,
        "demo card must have its own rule test: {name}"
    );
    id
}

fn starter(land: &str, spells: &[(&str, u8, u8)], catalog: &mut Catalog) -> Collection {
    let owned: Vec<_> = spells
        .iter()
        .map(|&(name, copies, price)| {
            let card = card(name);
            catalog.cards.push(Card {
                card,
                price,
                basic_land: false,
                run: None,
            });
            Stack { card, copies }
        })
        .collect();
    let mut deck = owned.clone();
    deck.push(Stack {
        card: card(land),
        copies: 17,
    });
    deck.sort_by_key(|stack| stack.card);
    Collection {
        formation: None,
        owned,
        deck,
    }
}

pub(in super::super) async fn publish_house(
    store: &Store,
    publisher: &Principal,
    cohort: Uuid,
    publication: &Publication,
) {
    let demo = super::enabled();
    for stage in 1..=if demo { 12 } else { 1 } {
        for (index, starter) in publication
            .starters
            .iter()
            .take(if demo { 3 } else { 1 })
            .enumerate()
        {
            let mut active = Vec::new();
            let rarity = if stage >= 7 {
                Rarity::Uncommon
            } else {
                Rarity::Common
            };
            if stage >= 2 {
                active.push(Definition {
                    family: Family::Heartwood,
                    rarity,
                });
            }
            if stage >= 4 {
                active.push(Definition {
                    family: Family::Grove,
                    rarity,
                });
            }
            if stage >= 10 {
                active.push(Definition {
                    family: Family::Compass,
                    rarity: Rarity::Common,
                });
            }
            let build = Build {
                formation: starter.formation.clone(),
                deck: starter.deck.clone(),
                active,
                potions: [0; 2],
            };
            store
                .publish_house(
                    publisher,
                    cohort,
                    stage,
                    u8::try_from(index + 1).unwrap(),
                    &build,
                )
                .await
                .unwrap();
        }
    }
}

#[test]
fn playable_demo_starters_use_tested_cards_and_exact_legal_collections() {
    let publication = playable(&support::publication().cohort.executor);
    for starter in &publication.starters {
        starter.validate(&publication.catalog).unwrap();
        assert_eq!(
            starter
                .deck
                .iter()
                .map(|row| u16::from(row.copies))
                .sum::<u16>(),
            40
        );
        assert_eq!(
            starter
                .owned
                .iter()
                .map(|row| u16::from(row.copies))
                .sum::<u16>(),
            23
        );
    }
}
