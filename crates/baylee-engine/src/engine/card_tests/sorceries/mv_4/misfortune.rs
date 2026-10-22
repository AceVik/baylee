//! `cards/sorceries/mv_4/misfortune.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::life;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn misfortune() -> CardIndex {
    card_index("e282eb5b-f9e6-4fb3-9edf-44e23359357d")
}

/// Misfortune ({1}{B}{R}{G} Sorcery, Partial):
/// - Mode 0: Put a +1/+1 counter on each creature you control and gain 4 life.
/// - Mode 1: Put a -1/-1 counter on each creature opponents control and deal 4 damage to each opponent.
#[test]
fn misfortune_both_modes_apply_counters_and_life_or_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    // Mode 0: +1/+1 counter on each creature you control and gain 4 life.
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), mountain(), forest(), swamp(), GRIZZLY_BEARS])
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[misfortune()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let life_before = life(&engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, misfortune());
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (3, 3));
    assert_eq!(life(&engine, p0), life_before + 4);

    // Mode 1: -1/-1 counter on each opponent creature and 4 damage to each opponent.
    let mut engine2 = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), mountain(), forest(), swamp(), GRIZZLY_BEARS])
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[misfortune()])
        .start();
    keep_mulligans(&mut engine2);
    reach_main_phase(&mut engine2, p0);

    let giant = on_battlefield(&engine2, p1, HILL_GIANT).unwrap();
    let opp_life_before = life(&engine2, p1);

    tap_all_mana(&mut engine2, p0);
    cast_with_floating(&mut engine2, p0, misfortune());
    engine2.apply(p0, PlayerAction::ChooseMode(1)).unwrap();
    pass_until(&mut engine2, stack_is_empty);

    assert_eq!(pt(&engine2, giant), (2, 2));
    assert_eq!(life(&engine2, p1), opp_life_before - 4);
}
