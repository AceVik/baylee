//! `cards/instants/mv_2/vitalizing_cascade.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn vitalizing_cascade() -> CardIndex {
    card_index("aaf176ff-646f-4182-a546-fe648498587d")
}

/// Vitalizing Cascade (`{X}{G}{W}`, `Coverage::Implemented`): you gain X plus
/// 3 life.
#[test]
fn vitalizing_cascade_gains_x_plus_3_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), plains(), forest(), plains()])
        .hand(0, &[vitalizing_cascade()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Cast with X = 2 (total cost 4 mana) -> gain 2 + 3 = 5 life (20 -> 25).
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, vitalizing_cascade());
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 25);
}
