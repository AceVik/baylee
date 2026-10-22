//! `cards/enchantments/mv_3/cycle_of_life.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{CYCLE_OF_LIFE, HILL_GIANT};

const P0: PlayerId = PlayerId::new(0);

/// Cycle of Life (5941374b-eeb8-4d0e-9367-f7f0f8b79739) — {1}{G}{G} Enchantment:
/// "Return this enchantment to its owner's hand: Target creature you cast this
/// turn has base power and toughness 0/1 until your next upkeep."
#[test]
fn cycle_of_life_returns_to_hand_and_sets_creature_to_zero_one_until_next_upkeep() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                CYCLE_OF_LIFE,
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, HILL_GIANT);
    pass_until(&mut engine, stack_is_empty);

    let giant = on_battlefield(&engine, P0, HILL_GIANT).unwrap();
    assert_eq!(pt(&engine, giant), (3, 3));

    activate(&mut engine, P0, CYCLE_OF_LIFE, 0);
    aim_at(&mut engine, P0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, CYCLE_OF_LIFE).is_none());
    assert!(in_hand(&engine, P0, CYCLE_OF_LIFE).is_some());
    assert_eq!(pt(&engine, giant), (0, 1));

    // At P0's next upkeep (turn 3), the 0/1 effect expires and Hill Giant is 3/3 again.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number == 3
            && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(pt(&engine, giant), (3, 3));
}
