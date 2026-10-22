//! `cards/creatures/mv_2/mogg_squad.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::power_toughness;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn mogg_squad() -> CardIndex {
    card_index("4dac142a-c8f8-45c8-85d3-40e170dee112")
}

/// Mogg Squad (`4dac142a-c8f8-45c8-85d3-40e170dee112`) — {1}{R} Creature — Goblin:
/// "This creature gets -1/-1 for each other creature on the battlefield."
#[test]
fn mogg_squad_gets_minus_one_minus_one_for_each_other_creature() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), index::GRIZZLY_BEARS])
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[mogg_squad()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, mogg_squad());
    pass_until(&mut engine, stack_is_empty);

    let id = on_battlefield(&engine, P0, mogg_squad()).unwrap();
    assert_eq!(power_toughness(&engine, id), (1, 1));
}
