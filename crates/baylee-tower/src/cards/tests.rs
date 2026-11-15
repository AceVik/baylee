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
