//! `cards/artifacts/mv_1/ivory_tower.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ivory Tower: "At the beginning of your upkeep, you gain X life, where X
/// is the number of cards in your hand minus 4."
///
/// The 2004 ruling is the floor: four cards gain nothing, seven gain three.
/// p1's Tower is the control for "your" — it watches its own controller's
/// upkeep and does not fire at this one.
#[test]
fn ivory_tower_gains_the_hand_minus_four_at_its_own_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for (cards, gain) in [(0usize, 0i32), (4, 0), (7, 3)] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[ivory_tower()])
            .hand(0, &vec![forest(); cards])
            .battlefield(1, &[ivory_tower()])
            .hand(1, &[forest(); 7])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        assert_eq!(
            life_of(&engine, p0),
            20 + gain,
            "a hand of {cards}: X is {cards} - 4, floored at zero"
        );
        assert_eq!(
            life_of(&engine, p1),
            20,
            "p1's Tower gains at p1's upkeep, not at p0's"
        );
    }
}
