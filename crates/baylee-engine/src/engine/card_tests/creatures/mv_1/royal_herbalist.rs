//! `cards/creatures/mv_1/royal_herbalist.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::life;

fn royal_herbalist() -> CardIndex {
    card_index("b3baf498-3be7-4a22-8fed-806d8d8ac748")
}

/// Royal Herbalist ({W} 1/1 Creature — Human Cleric, Partial):
/// "{2}, Exile the top card of your library: You gain 1 life."
#[test]
fn royal_herbalist_activated_ability_gains_one_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[royal_herbalist(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let herbalist = on_battlefield(&engine, p0, royal_herbalist()).unwrap();
    let life_before = life(&engine, p0);

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: herbalist,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(life(&engine, p0), life_before + 1);
}
