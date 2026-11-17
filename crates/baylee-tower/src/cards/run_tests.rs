use super::*;
use baylee_core::color::{Color, ColorSet};

pub(crate) fn fixture() -> (Catalog, Collection) {
    let mut cards: Vec<_> = Color::ALL
        .into_iter()
        .enumerate()
        .map(|(index, color)| Card {
            card: CardIndex::new(u32::try_from(index).unwrap()),
            price: 0,
            basic_land: true,
            run: Some(RunCard {
                presentation: None,
                identity: ColorSet::of(color),
                commander: false,
            }),
        })
        .collect();
    for (index, color, commander) in [
        (5, Some(Color::Green), true),
        (6, Some(Color::Green), false),
        (7, None, false),
        (8, Some(Color::Red), false),
        (9, Some(Color::Green), false),
    ] {
        cards.push(Card {
            card: CardIndex::new(index),
            price: 4,
            basic_land: false,
            run: Some(RunCard {
                presentation: None,
                identity: color.map_or(ColorSet::EMPTY, ColorSet::of),
                commander,
            }),
        });
    }
    let catalog = Catalog { cards };
    let collection = Collection {
        formation: Some(Formation {
            commander: CardIndex::new(5),
            sideboard: vec![stack(9, 1)],
        }),
        owned: [5, 6, 7, 9].map(|id| stack(id, 1)).into(),
        deck: vec![stack(4, 37), stack(6, 1), stack(7, 1)],
    };
    (catalog, collection)
}

fn stack(card: u32, copies: u8) -> Stack {
    Stack {
        card: CardIndex::new(card),
        copies,
    }
}

#[test]
fn forty_including_commander_free_basics_and_five_sideboard_are_enforced() {
    let (catalog, collection) = fixture();
    collection.validate(&catalog).unwrap();
    let formation = collection.formation.clone().unwrap();
    let all_basics = collection
        .refitted(&catalog, vec![stack(4, 39)], formation.clone())
        .unwrap();
    assert_eq!(all_basics.owned, collection.owned);
    for count in [0, 38, 40, 60, 255] {
        assert!(
            collection
                .refitted(&catalog, vec![stack(4, count)], formation.clone())
                .is_err()
        );
    }
    let mut side = formation;
    side.sideboard = vec![stack(4, 5)];
    collection
        .refitted(&catalog, collection.deck.clone(), side.clone())
        .unwrap();
    side.sideboard[0].copies = 6;
    assert!(
        collection
            .refitted(&catalog, collection.deck.clone(), side)
            .is_err()
    );
}

#[test]
fn ownership_identity_leader_and_cross_zone_singleton_are_authoritative() {
    let (catalog, collection) = fixture();
    let mut forged = collection.clone();
    forged.owned.retain(|stack| stack.card != CardIndex::new(6));
    assert!(forged.validate(&catalog).is_err());
    let mut off_color = collection.bought(&catalog, CardIndex::new(8)).unwrap();
    off_color.deck[1] = stack(8, 1);
    assert!(off_color.validate(&catalog).is_err());
    let mut duplicate = collection.clone();
    duplicate.formation.as_mut().unwrap().sideboard = vec![stack(6, 1)];
    assert!(duplicate.validate(&catalog).is_err());
    let mut leader_in_deck = collection.clone();
    leader_in_deck.deck[1] = stack(5, 1);
    assert!(leader_in_deck.validate(&catalog).is_err());
    let mut wrong_leader = collection.clone();
    wrong_leader.formation.as_mut().unwrap().commander = CardIndex::new(9);
    assert!(wrong_leader.validate(&catalog).is_err());
    let mut repeated_row = collection.clone();
    repeated_row.deck[0].copies -= 1;
    repeated_row.deck.push(stack(4, 1));
    assert!(repeated_row.validate(&catalog).is_err());
    assert!(collection.bought(&catalog, CardIndex::new(5)).is_err());
}

#[test]
fn catalog_requires_closed_metadata_and_exactly_five_distinct_basic_colors() {
    let (catalog, _) = fixture();
    let mut mixed = catalog.clone();
    mixed.cards[2].run = None;
    assert!(mixed.validate().is_err());
    let mut missing = catalog.clone();
    missing.cards.remove(0);
    assert!(missing.validate().is_err());
    let mut duplicate = catalog.clone();
    duplicate.cards[0].run = duplicate.cards[1].run;
    assert!(duplicate.validate().is_err());
    let mut invalid = catalog;
    invalid.cards[5].run.as_mut().unwrap().identity = serde_json::from_str("32").unwrap();
    assert!(invalid.validate().is_err());
}

#[test]
fn legacy_catalog_and_collection_do_not_serialize_new_format_fields() {
    let card = Card {
        card: CardIndex::new(0),
        price: 0,
        basic_land: true,
        run: None,
    };
    assert_eq!(
        serde_json::to_string(&card).unwrap(),
        r#"{"card":0,"price":0,"basic_land":true}"#
    );
    let collection = Collection {
        formation: None,
        owned: vec![],
        deck: vec![stack(0, 40)],
    };
    let encoded = serde_json::to_string(&collection).unwrap();
    assert!(!encoded.contains("formation"));
    assert_eq!(
        serde_json::from_str::<Collection>(&encoded).unwrap(),
        collection
    );
}

#[test]
fn run_publications_require_at_least_five_distinct_valid_commanders() {
    let (mut catalog, collection) = fixture();
    let mut starters = vec![collection.clone()];
    for id in 10..14 {
        catalog.cards.push(Card {
            card: CardIndex::new(id),
            ..catalog.cards[5]
        });
        let mut starter = collection.clone();
        starter.formation.as_mut().unwrap().commander = CardIndex::new(id);
        starter.owned[0].card = CardIndex::new(id);
        starters.push(starter);
    }
    catalog.validate_starters(&starters).unwrap();
    assert!(catalog.validate_starters(&starters[..4]).is_err());
    assert!(catalog.validate_starters(&vec![collection; 5]).is_err());
    starters[4].deck[0].copies = 38;
    assert!(catalog.validate_starters(&starters).is_err());
}

#[test]
fn selling_unused_cards_protects_commander_library_sideboard_and_basics() {
    use crate::cards::{CardRarity, Presentation};
    use baylee_core::preset::Finish;

    let (mut catalog, mut collection) = fixture();
    for (i, rarity) in [
        CardRarity::Common,
        CardRarity::Common,
        CardRarity::Common,
        CardRarity::Common,
        CardRarity::Common,
        CardRarity::Mythic,
        CardRarity::Common,
        CardRarity::Uncommon,
        CardRarity::Rare,
        CardRarity::Mythic,
    ]
    .into_iter()
    .enumerate()
    {
        catalog.cards[i].run.as_mut().unwrap().presentation = Some(Presentation {
            printing: uuid::Uuid::from_u128(u128::try_from(i + 1).unwrap()),
            finish: Finish::Normal,
            rarity,
        });
    }
    assert_eq!(catalog.card_sale_gold(CardIndex::new(4)), None);
    assert_eq!(catalog.card_sale_gold(CardIndex::new(999)), None);
    assert_eq!(catalog.card_sale_gold(CardIndex::new(6)), Some(1));
    assert_eq!(catalog.card_sale_gold(CardIndex::new(7)), Some(2));
    assert_eq!(catalog.card_sale_gold(CardIndex::new(8)), Some(3));
    assert_eq!(catalog.card_sale_gold(CardIndex::new(9)), Some(4));

    // Commander (5), main-library cards (6, 7), sideboard card (9) and basic (4) are all refused.
    for protected in [4, 5, 6, 7, 8, 9] {
        assert!(
            collection
                .sold(&catalog, CardIndex::new(protected))
                .is_err()
        );
    }

    // Loot an off-color Rare (8) into `owned` without adding it to library or sideboard.
    collection = collection.bought(&catalog, CardIndex::new(8)).unwrap();
    let (after_rare, gold) = collection.sold(&catalog, CardIndex::new(8)).unwrap();
    assert_eq!(gold, 3);
    assert_eq!(after_rare.copies(CardIndex::new(8)), 0);
    assert!(after_rare.sold(&catalog, CardIndex::new(8)).is_err());

    // Removing card 9 from sideboard frees it to be sold for Mythic value (4g).
    let mut clear_side = after_rare.formation.clone().unwrap();
    clear_side.sideboard.clear();
    let freed = after_rare
        .refitted(&catalog, after_rare.deck.clone(), clear_side)
        .unwrap();
    let (after_mythic, mythic_gold) = freed.sold(&catalog, CardIndex::new(9)).unwrap();
    assert_eq!(mythic_gold, 4);
    assert_eq!(after_mythic.copies(CardIndex::new(9)), 0);
}
