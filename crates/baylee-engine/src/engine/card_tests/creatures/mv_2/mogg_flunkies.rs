//! `cards/creatures/mv_2/mogg_flunkies.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::power_toughness;

const P0: PlayerId = PlayerId::new(0);

fn mogg_flunkies() -> CardIndex {
    card_index("b92fc9c1-7264-4894-90b8-a193039489c5")
}

/// Mogg Flunkies (`b92fc9c1-7264-4894-90b8-a193039489c5`) — {1}{R} 3/3 Creature — Goblin (Partial).
#[test]
fn mogg_flunkies_enters_as_three_three_goblin() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[mogg_flunkies()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, mogg_flunkies());
    pass_until(&mut engine, stack_is_empty);

    let id = on_battlefield(&engine, P0, mogg_flunkies()).unwrap();
    assert_eq!(power_toughness(&engine, id), (3, 3));
}
