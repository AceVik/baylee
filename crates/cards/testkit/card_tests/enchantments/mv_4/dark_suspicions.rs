//! `cards/enchantments/mv_4/dark_suspicions.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn dark_suspicions() -> CardIndex {
    card_index("cf5cf174-d976-4cd8-9721-c30fe784fb70")
}

/// On each opponent's upkeep, that opponent loses `cards_in_their_hand` minus
/// `cards_in_your_hand` (clamped to 0); your own upkeep does not drain you.
#[test]
fn dark_suspicions_drains_opponents_with_more_cards_in_hand_on_their_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[dark_suspicions()],
        &[],
        &[forest()],
        &[forest(), forest(), forest(), forest()],
    );

    // P1 enters upkeep with 4 cards vs P0's 1 -> loses 3 life (20 -> 17), draws to 5.
    reach_their_main_phase(&mut e, p1);
    assert_eq!(lk::life(&e, p1), 17);

    // P0's own upkeep with cards in hand never loses life.
    reach_their_main_phase(&mut e, p0);
    assert_eq!(lk::life(&e, p0), 20);

    // Now simulate P1 having FEWER cards (play/empty or add to P0's hand via draw): cast Tome/Wait: cast P1's forests down?
    // Even simpler: P1 already has 5 cards, let's give P0 more cards or check P0 upkeep -> P0 cast nothing so P0 has 2 cards, P1 has 5 -> next P1 upkeep P1 loses 5 - 2 = 3.
    // Wait: let's test <=0 clamp cleanly by passing to P1 main, discarding? Easier: cast Raise? Let's just use Dark Rituals!
    // Check current life: P1 will be at 14 if 5 vs 2; instead, test clamp by giving P0 6 Forests in setup hand & checking clamp on P1's first upkeep!
    assert_eq!(lk::life(&e, p0), 20);
}

#[test]
fn dark_suspicions_deals_zero_when_opponent_has_fewer_or_equal_cards() {
    let (_p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[dark_suspicions()],
        &[],
        &[forest(), forest(), forest(), forest()],
        &[forest()],
    );
    reach_their_main_phase(&mut e, p1);
    assert_eq!(lk::life(&e, p1), 20);
}
