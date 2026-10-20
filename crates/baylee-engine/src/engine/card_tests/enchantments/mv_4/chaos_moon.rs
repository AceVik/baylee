//! `cards/enchantments/mv_4/chaos_moon.rs`, played.
//!
//! Oracle:
//! - "At the beginning of each upkeep, count the number of permanents. If the number is odd, until end of turn, red creatures get +1/+1 and whenever a player taps a Mountain for mana, that player adds an additional {R}. If the number is even, until end of turn, red creatures get -1/-1 and if a player taps a Mountain for mana, that Mountain produces colorless mana instead of any other type."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "728e2960-1d85-4b3f-8b5b-915f9b9ffd2f"
fn chaos_moon() -> CardIndex {
    card_index("728e2960-1d85-4b3f-8b5b-915f9b9ffd2f")
}

#[test]
fn chaos_moon_upkeep_trigger_modifies_red_creatures_until_end_of_turn() {
    let mut engine = lk::game(
        SEED,
        &[index::HILL_GIANT, index::GRIZZLY_BEARS],
        &[],
        &[chaos_moon()],
        &[],
    );
    let giant = lk::obj(&engine, P0, index::HILL_GIANT);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 3), (ManaColor::Red, 1)],
    );
    cast_with_floating(&mut engine, P0, chaos_moon());
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });

    assert_eq!(
        pt(&engine, giant),
        (4, 4),
        "red creature gets +1/+1 until end of turn after upkeep trigger"
    );
    assert_eq!(pt(&engine, bears), (2, 2), "non-red creature is unaffected");
}
