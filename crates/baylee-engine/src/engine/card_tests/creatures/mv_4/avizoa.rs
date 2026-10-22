//! `cards/creatures/mv_4/avizoa.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn avizoa() -> CardIndex {
    card_index("91520db8-2d50-4c17-9f0e-2f92f63285f7")
}

/// `Avizoa` ({3}{U}, 2/2 Creature — Jellyfish, `Coverage::Partial`) activates
/// `{0}` once each turn to get +2/+2 until end of turn.
#[test]
fn avizoa_pumps_plus_two_plus_two_once_each_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[avizoa()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, avizoa());
    pass_until(&mut engine, stack_is_empty);
    let id = on_battlefield(&engine, p0, avizoa()).unwrap();

    activate(&mut engine, p0, avizoa(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, id), (4, 4));
}
