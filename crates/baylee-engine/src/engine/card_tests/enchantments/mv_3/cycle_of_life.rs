//! `cards/enchantments/mv_3/cycle_of_life.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

const P0: PlayerId = PlayerId::new(0);

fn cycle_of_life() -> CardIndex {
    card_index("5941374b-eeb8-4d0e-9367-f7f0f8b79739")
}

/// Cycle of Life (5941374b-eeb8-4d0e-9367-f7f0f8b79739) — {1}{G}{G} Enchantment:
/// "Return this enchantment to its owner's hand: Target creature you cast this
/// turn has base power and toughness 0/1 until your next upkeep."
#[test]
fn cycle_of_life_returns_to_hand_and_sets_creature_to_zero_one_until_next_upkeep() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                cycle_of_life(),
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

    activate(&mut engine, P0, cycle_of_life(), 0);
    aim_at(&mut engine, P0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, cycle_of_life()).is_none());
    assert!(in_hand(&engine, P0, cycle_of_life()).is_some());
    assert_eq!(pt(&engine, giant), (0, 1));

    // At P0's next upkeep (turn 3), the 0/1 effect expires and Hill Giant is 3/3 again.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number == 3
            && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(pt(&engine, giant), (3, 3));
}

/// "Target creature you cast this turn": a creature that was already on the
/// battlefield is not on the menu, and it keeps its size when the ability
/// resolves on the one that was just cast.
#[test]
fn cycle_of_life_cannot_aim_at_a_creature_that_was_already_there() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                cycle_of_life(),
                GRIZZLY_BEARS,
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

    let old = on_battlefield(&engine, P0, GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, HILL_GIANT);
    pass_until(&mut engine, stack_is_empty);
    let giant = on_battlefield(&engine, P0, HILL_GIANT).unwrap();

    activate(&mut engine, P0, cycle_of_life(), 0);
    let menu = aim_at(&mut engine, P0, giant);
    assert!(menu.contains(&giant), "the creature just cast: {menu:?}");
    assert!(
        !menu.contains(&old),
        "a creature that did not enter this turn is not a target: {menu:?}"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, giant), (0, 1));
    assert_eq!(pt(&engine, old), (2, 2));
}
