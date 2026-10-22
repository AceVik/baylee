//! `cards/sorceries/mv_4/gerrard_s_wisdom.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GERRARD_S_WISDOM;

/// `Gerrard's Wisdom` ({2}{W}{W}, Sorcery) gains 2 life for each card in your
/// hand.
#[test]
fn gerrard_s_wisdom_gains_2_life_for_each_card_in_your_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[GERRARD_S_WISDOM, plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, GERRARD_S_WISDOM);
    let hand_size = i32::try_from(engine.state().zones.list(ZoneLocation::Hand(p0)).len()).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 20 + 2 * hand_size);
}
