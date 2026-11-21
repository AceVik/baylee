//! `cards/artifacts/mv_4/skullcage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn skullcage() -> CardIndex {
    card_index("a96387fb-ff10-4299-8f42-aa61da693d07")
}

/// Runs Skullcage up to p1's upkeep with `hand_size` cards in p1's hand and
/// returns p1's life total after the upkeep trigger resolves.
fn p1_life_after_upkeep_with_hand_size(hand_size: usize) -> i32 {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[skullcage()])
        .hand(1, &vec![forest(); hand_size])
        .start();
    keep_mulligans(&mut engine);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        hand_size
    );
    reach_their_main_phase(&mut engine, p1);
    life_of(&engine, p1)
}

/// Skullcage (a96387fb-ff10-4299-8f42-aa61da693d07) — {4} Artifact:
/// "At the beginning of each opponent's upkeep, this artifact deals 2 damage
///  to that player unless they have exactly three or exactly four cards in
///  hand."
///
/// Deals 2 damage at hand sizes 2 and 5, deals 0 at exact safe boundaries 3
/// and 4, and never damages its controller on their own upkeep.
#[test]
fn skullcage_hits_opponent_at_two_and_five_spares_three_and_four_and_never_hits_you() {
    assert_eq!(
        p1_life_after_upkeep_with_hand_size(2),
        18,
        "2 cards in hand => takes 2 damage"
    );
    assert_eq!(
        p1_life_after_upkeep_with_hand_size(3),
        20,
        "exactly 3 cards in hand => takes 0 damage"
    );
    assert_eq!(
        p1_life_after_upkeep_with_hand_size(4),
        20,
        "exactly 4 cards in hand => takes 0 damage"
    );
    assert_eq!(
        p1_life_after_upkeep_with_hand_size(5),
        18,
        "5 cards in hand => takes 2 damage"
    );

    // Controller's own upkeep (7 cards in hand!) must NOT deal damage to p0.
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[skullcage()])
        .hand(0, &[forest(); 7])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert_eq!(
        life_of(&engine, p0),
        20,
        "Skullcage triggers only on opponents' upkeeps"
    );
}
