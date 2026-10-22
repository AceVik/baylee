//! `cards/creatures/mv_5/magmasaur.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::power_toughness;

const P0: PlayerId = PlayerId::new(0);

fn magmasaur() -> CardIndex {
    card_index("da38bfdf-b4c8-4ffe-ad53-b42c317cae5f")
}

/// Magmasaur (`da38bfdf-b4c8-4ffe-ad53-b42c317cae5f`) — {3}{R}{R} Creature — Elemental Dinosaur (Partial):
/// "This creature enters with five +1/+1 counters on it."
#[test]
fn magmasaur_enters_with_five_plus_one_plus_one_counters() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[magmasaur()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, magmasaur());
    pass_until(&mut engine, stack_is_empty);

    let id = on_battlefield(&engine, P0, magmasaur()).unwrap();
    assert_eq!(power_toughness(&engine, id), (5, 5));
}
