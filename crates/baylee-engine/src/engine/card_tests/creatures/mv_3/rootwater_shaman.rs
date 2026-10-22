//! `cards/creatures/mv_3/rootwater_shaman.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn rootwater_shaman() -> CardIndex {
    card_index("8a449915-8bd7-4a9c-abb1-a95852f33695")
}

/// `Rootwater Shaman` ({2}{U}, 2/2 Creature — Merfolk Shaman, `Coverage::Partial`):
/// enters the battlefield as a 2/2.
#[test]
fn rootwater_shaman_casts_and_enters_as_a_two_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[rootwater_shaman()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, rootwater_shaman());
    pass_until(&mut engine, stack_is_empty);

    let shaman = on_battlefield(&engine, p0, rootwater_shaman()).unwrap();
    let ch = engine.state().object(shaman).unwrap().characteristics();
    assert_eq!((ch.power, ch.toughness), (Some(2), Some(2)));
}
