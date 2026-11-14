use super::*;
use crate::{cards::Card, economy::Merchandise, items::Potion};
use baylee_core::ids::CardIndex;

fn setup() -> (Catalog, Run) {
    let catalog = Catalog {
        cards: vec![
            Card {
                card: CardIndex(1),
                price: 0,
                basic_land: true,
            },
            Card {
                card: CardIndex(2),
                price: 4,
                basic_land: false,
            },
            Card {
                card: CardIndex(3),
                price: 7,
                basic_land: false,
            },
        ],
    };
    let starter = Collection {
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
    };
    let run = Run::begin(&catalog, starter, &[1; 32]).unwrap();
    (catalog, run)
}

fn slot(run: &Run, wanted: Goods) -> u8 {
    run.shop
        .offers
        .iter()
        .position(|offer| *offer == wanted)
        .unwrap() as u8
}

fn awaiting(run: &Run, catalog: &Catalog) -> Run {
    run.queue(run.revision, catalog)
        .unwrap()
        .execution(Phase::Playing, catalog)
        .unwrap()
        .execution(Phase::Verifying, catalog)
        .unwrap()
}

#[test]
fn revision_and_saved_stock_prevent_a_duplicate_purchase_without_partial_spending() {
    let (catalog, initial) = setup();
    let healing = slot(
        &initial,
        Goods::Item {
            item: Merchandise::Potion {
                potion: Potion::Healing,
            },
        },
    );
    let bought = initial.buy(initial.revision, healing, &catalog).unwrap();
    assert_eq!(bought.economy.gold, 9);
    assert_eq!(bought.economy.potions, [1, 0]);
    assert_eq!(
        bought.buy(initial.revision, healing, &catalog),
        Err(Refusal::Stale)
    );
    assert_eq!(
        bought.buy(bought.revision, healing, &catalog),
        Err(Refusal::Stale)
    );
    assert_eq!(initial.economy, Economy::default());
    let invalid = bought.buy(bought.revision, 255, &catalog);
    assert_eq!(invalid, Err(Refusal::Stale));
}

#[test]
fn card_purchase_charges_once_and_does_not_silently_replace_the_deck() {
    let (catalog, initial) = setup();
    let card = slot(&initial, Goods::Card { card: CardIndex(3) });
    let bought = initial.buy(0, card, &catalog).unwrap();
    assert_eq!(bought.economy.gold, 5);
    assert_eq!(bought.cards.copies(CardIndex(3)), 1);
    assert_eq!(bought.cards.deck, initial.cards.deck);
    let mut deck = bought.cards.deck.clone();
    deck[0].copies -= 1;
    deck.push(Stack {
        card: CardIndex(3),
        copies: 1,
    });
    let edited = bought.edit(bought.revision, deck, &catalog).unwrap();
    assert_eq!(edited.economy, bought.economy);
    assert_eq!(edited.cards.owned, bought.cards.owned);
}

#[test]
fn all_shopping_stops_after_reservation_and_voids_cannot_reroll_or_grant_gold() {
    let (catalog, initial) = setup();
    let queued = initial.queue(0, &catalog).unwrap();
    assert_eq!(
        queued.buy(queued.revision, 0, &catalog),
        Err(Refusal::Unavailable)
    );
    assert_eq!(
        queued.edit(queued.revision, initial.cards.deck.clone(), &catalog),
        Err(Refusal::Unavailable)
    );
    assert_eq!(
        queued.reroll(queued.revision, &catalog, &[4; 32]),
        Err(Refusal::Unavailable)
    );
    let playing = queued.execution(Phase::Playing, &catalog).unwrap();
    assert_eq!(
        playing.abandon(playing.revision, &catalog),
        Err(Refusal::Unavailable)
    );
    let voided = playing.voided(&catalog).unwrap();
    assert_eq!(voided.phase, Phase::Queued);
    assert_eq!(voided.economy, initial.economy);
    assert_eq!(voided.cards, initial.cards);
    assert_eq!(voided.shop, initial.shop);
    assert_eq!((voided.wins, voided.defeats, voided.turns), (0, 0, 0));
}

#[test]
fn only_verified_terminal_games_advance_stage_and_repeated_results_cannot_pay_twice() {
    let (catalog, initial) = setup();
    assert_eq!(
        initial.verified(Outcome::Victory, 10, [0, 0], &catalog, &[1; 32]),
        Err(Refusal::Unavailable)
    );
    let verifying = awaiting(&initial, &catalog);
    assert_eq!(
        verifying.verified(Outcome::Victory, 0, [0, 0], &catalog, &[1; 32]),
        Err(Refusal::Invalid)
    );
    assert_eq!(
        verifying.verified(Outcome::Victory, 10, [1, 0], &catalog, &[1; 32]),
        Err(Refusal::Invalid)
    );
    let won = verifying
        .verified(Outcome::Victory, 10, [0, 0], &catalog, &[1; 32])
        .unwrap();
    assert_eq!((won.stage(), won.wins, won.defeats), (2, 1, 0));
    assert_eq!(won.economy.gold, STARTING_GOLD + DUEL_INCOME);
    assert_eq!(won.shop.rerolls, 0);
    assert_eq!(
        won.verified(Outcome::Victory, 10, [0, 0], &catalog, &[1; 32]),
        Err(Refusal::Unavailable)
    );
    let drawn = awaiting(&won, &catalog)
        .verified(Outcome::Draw, 20, [0, 0], &catalog, &[1; 32])
        .unwrap();
    assert_eq!((drawn.stage(), drawn.wins, drawn.defeats), (3, 1, 1));
    assert_eq!(drawn.economy.gold, STARTING_GOLD + 2 * DUEL_INCOME);
}

#[test]
fn finite_runs_stop_at_five_wins_or_three_unsuccessful_duels_and_keep_consumption() {
    let (catalog, initial) = setup();
    for (outcome, count, phase) in [
        (Outcome::Victory, TARGET_WINS, Phase::Conquered),
        (Outcome::Defeat, DEFEAT_LIMIT, Phase::Exhausted),
    ] {
        let potion = slot(
            &initial,
            Goods::Item {
                item: Merchandise::Potion {
                    potion: Potion::Mana,
                },
            },
        );
        let mut run = initial.buy(0, potion, &catalog).unwrap();
        for duel in 0..count {
            run = awaiting(&run, &catalog)
                .verified(outcome, 15, [0, u8::from(duel == 0)], &catalog, &[1; 32])
                .unwrap();
        }
        assert_eq!(run.phase, phase);
        assert_eq!(run.economy.potions, [0, 0]);
        assert_eq!(run.turns, u32::from(count) * 15);
        assert_eq!(run.queue(run.revision, &catalog), Err(Refusal::Unavailable));
    }
}

#[test]
fn rerolls_cost_increasing_gold_are_order_independent_and_retain_stock_until_requested() {
    let (mut catalog, initial) = setup();
    let once = initial.reroll(0, &catalog, &[1; 32]).unwrap();
    assert_eq!((once.economy.gold, once.shop.rerolls), (10, 1));
    let twice = once.reroll(once.revision, &catalog, &[1; 32]).unwrap();
    assert_eq!((twice.economy.gold, twice.shop.rerolls), (7, 2));
    catalog.cards.reverse();
    let same = Shop::generate(&[1; 32], 1, 2, &catalog, &twice.economy, &twice.cards).unwrap();
    assert_eq!(twice.shop, same);
    assert_eq!(once.reroll(0, &catalog, &[1; 32]), Err(Refusal::Stale));
    let last = twice.reroll(twice.revision, &catalog, &[1; 32]).unwrap();
    assert_eq!(
        last.reroll(last.revision, &catalog, &[1; 32]),
        Err(Refusal::Gold)
    );
}

#[test]
fn forged_income_terminal_counts_shop_bits_and_revision_overflow_are_rejected() {
    let (catalog, initial) = setup();
    let mut bad = initial.clone();
    bad.economy.gold += 1;
    assert_eq!(bad.validate(&catalog), Err(Refusal::Invalid));
    bad = initial.clone();
    bad.phase = Phase::Conquered;
    assert_eq!(bad.validate(&catalog), Err(Refusal::Invalid));
    bad = initial.clone();
    bad.wins = 255;
    bad.defeats = 255;
    assert_eq!(bad.validate(&catalog), Err(Refusal::Invalid));
    bad = initial.clone();
    bad.shop.sold = 255;
    assert_eq!(bad.validate(&catalog), Err(Refusal::Invalid));
    bad = initial;
    bad.revision = u32::MAX;
    assert_eq!(bad.queue(u32::MAX, &catalog), Err(Refusal::Invalid));
}
