use super::*;

fn setup() -> (Catalog, Collection) {
    (
        Catalog {
            cards: vec![
                Card {
                    card: CardIndex(1),
                    price: 0,
                    basic_land: true,
                    run: None,
                },
                Card {
                    card: CardIndex(2),
                    price: 4,
                    basic_land: false,
                    run: None,
                },
                Card {
                    card: CardIndex(3),
                    price: 7,
                    basic_land: false,
                    run: None,
                },
            ],
        },
        Collection {
            formation: None,
            owned: vec![Stack {
                card: CardIndex(2),
                copies: 4,
            }],
            deck: vec![
                Stack {
                    card: CardIndex(1),
                    copies: 36,
                },
                Stack {
                    card: CardIndex(2),
                    copies: 4,
                },
            ],
        },
    )
}

#[test]
fn purchased_cards_enter_the_collection_then_a_legal_deck_edit() {
    let (catalog, old) = setup();
    let owned = old.bought(&catalog, CardIndex(3)).unwrap();
    assert_eq!(owned.deck, old.deck);
    let mut edit = old.deck.clone();
    edit[0].copies -= 1;
    edit.push(Stack {
        card: CardIndex(3),
        copies: 1,
    });
    assert!(old.edited(&catalog, edit.clone()).is_err());
    let edited = owned.edited(&catalog, edit).unwrap();
    edited.validate(&catalog).unwrap();
    assert_eq!(edited.owned, owned.owned);
    assert_eq!(old.copies(CardIndex(3)), 0);
}

#[test]
fn free_lands_and_duplicate_rows_cannot_create_paid_copies_or_bypass_deck_limits() {
    let (catalog, old) = setup();
    assert_eq!(
        old.bought(&catalog, CardIndex(1)),
        Err(Refusal::Unavailable)
    );
    assert_eq!(old.bought(&catalog, CardIndex(2)), Err(Refusal::Capacity));
    assert_eq!(
        old.bought(&catalog, CardIndex(999)),
        Err(Refusal::Unavailable)
    );
    let mut duplicated = old.deck.clone();
    duplicated.push(Stack {
        card: CardIndex(2),
        copies: 1,
    });
    assert!(old.edited(&catalog, duplicated).is_err());
    assert!(
        old.edited(
            &catalog,
            vec![Stack {
                card: CardIndex(1),
                copies: 61
            }]
        )
        .is_err()
    );
    assert!(
        old.edited(
            &catalog,
            vec![Stack {
                card: CardIndex(1),
                copies: 39
            }]
        )
        .is_err()
    );
    assert!(
        old.edited(
            &catalog,
            vec![Stack {
                card: CardIndex(1),
                copies: 60
            }]
        )
        .is_ok()
    );
    let mut broken = old.clone();
    broken.owned.push(Stack {
        card: CardIndex(2),
        copies: 4,
    });
    assert_eq!(broken.validate(&catalog), Err(Refusal::Invalid));
}

#[test]
fn catalog_rejects_duplicate_identities_free_nonbasics_and_paid_basic_lands() {
    let (mut catalog, _) = setup();
    catalog.cards.push(catalog.cards[0]);
    assert_eq!(catalog.validate(), Err(Refusal::Invalid));
    catalog.cards.pop();
    catalog.cards[1].price = 0;
    assert_eq!(catalog.validate(), Err(Refusal::Invalid));
    catalog.cards[1].price = 4;
    catalog.cards[0].price = 1;
    assert_eq!(catalog.validate(), Err(Refusal::Invalid));
}

fn stack(card: u32, copies: u8) -> Stack {
    Stack {
        card: CardIndex(card),
        copies,
    }
}

#[test]
fn a_bought_copy_joins_its_stack_one_at_a_time_up_to_the_cap() {
    let (catalog, mut held) = setup();
    for copies in 1..=COPIES {
        held = held.bought(&catalog, CardIndex(3)).unwrap();
        assert_eq!(held.owned, [stack(2, 4), stack(3, copies)]);
        assert_eq!(held.copies(CardIndex(3)), copies);
    }
    for (card, refusal) in [
        (3, Refusal::Capacity),
        // A basic land is never sold, and a card outside the catalog never offered.
        (1, Refusal::Unavailable),
        (9, Refusal::Unavailable),
    ] {
        assert_eq!(held.bought(&catalog, CardIndex(card)), Err(refusal));
    }
}

#[test]
fn a_bought_card_takes_its_place_in_card_order() {
    let (catalog, mut held) = setup();
    held.owned = vec![stack(3, 1)];
    held.deck = vec![stack(1, 40)];
    let held = held.bought(&catalog, CardIndex(2)).unwrap();
    assert_eq!(held.owned, [stack(2, 1), stack(3, 1)]);
}

#[test]
fn a_deck_is_legal_from_forty_cards_to_sixty() {
    let (catalog, held) = setup();
    for (lands, legal) in [(35, false), (36, true), (56, true), (57, false)] {
        assert_eq!(
            held.edited(&catalog, vec![stack(1, lands), stack(2, 4)])
                .is_ok(),
            legal,
            "{lands} lands and four spells"
        );
    }
    // One row alone may be the whole deck, and no more than that.
    assert!(held.edited(&catalog, vec![stack(1, 60)]).is_ok());
    assert!(held.edited(&catalog, vec![stack(1, 61)]).is_err());
    // An empty row is not a row.
    assert!(
        held.edited(&catalog, vec![stack(1, 40), stack(2, 0)])
            .is_err()
    );
}

#[test]
fn an_edit_comes_back_in_card_order_and_leaves_the_inventory_alone() {
    let (catalog, held) = setup();
    let edited = held
        .edited(&catalog, vec![stack(2, 4), stack(1, 40)])
        .unwrap();
    assert_eq!(edited.deck, [stack(1, 40), stack(2, 4)]);
    assert_eq!(edited.owned, held.owned);
}

#[test]
fn a_row_count_over_the_deck_bound_is_refused_before_it_is_read() {
    let (catalog, held) = setup();
    // Sixty rows are read, and fail on what they say; sixty-one are not.
    let rows = |n: u32| (0..n).map(|card| stack(card + 100, 1)).collect::<Vec<_>>();
    assert_eq!(held.edited(&catalog, rows(60)), Err(Refusal::Unavailable));
    assert_eq!(held.edited(&catalog, rows(61)), Err(Refusal::Capacity));
    let saved = |deck| Collection {
        deck,
        ..held.clone()
    };
    assert_eq!(
        saved(rows(60)).validate(&catalog),
        Err(Refusal::Unavailable)
    );
    assert_eq!(saved(rows(61)).validate(&catalog), Err(Refusal::Capacity));
}

#[test]
fn an_owned_row_count_over_the_catalog_bound_is_refused_before_it_is_read() {
    let (catalog, held) = setup();
    let owned = |n: usize| Collection {
        owned: vec![stack(100, 1); n],
        ..held.clone()
    };
    assert_eq!(
        owned(MAX_CATALOG).validate(&catalog),
        Err(Refusal::Unavailable)
    );
    assert_eq!(
        owned(MAX_CATALOG + 1).validate(&catalog),
        Err(Refusal::Capacity)
    );
}

#[test]
fn a_legacy_catalog_holds_one_to_a_hundred_and_twenty_eight_cards() {
    let of = |n: u32| Catalog {
        cards: (0..n)
            .map(|card| Card {
                card: CardIndex(card),
                price: 4,
                basic_land: false,
                run: None,
            })
            .collect(),
    };
    for (n, legal) in [(0, false), (1, true), (128, true), (129, false)] {
        assert_eq!(of(n).validate().is_ok(), legal, "{n} cards");
    }
    for (price, legal) in [(0, false), (1, false), (2, true), (12, true), (13, false)] {
        let mut priced = of(1);
        priced.cards[0].price = price;
        assert_eq!(priced.validate().is_ok(), legal, "a card for {price}");
    }
}
