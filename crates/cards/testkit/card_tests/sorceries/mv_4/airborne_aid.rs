//! `cards/sorceries/mv_4/airborne_aid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn airborne_aid_draws_a_card_for_each_bird_on_the_battlefield() {
    let p0 = PlayerId::new(0);
    let aid = card_index("7f715f72-5444-4ab3-a6ca-3d5bfd0d4851");
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[aven_flock(), island(), island(), island(), island()])
        .battlefield(1, &[keeneye_aven()])
        .hand(0, &[aid])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_from_hand(&mut engine, p0, aid);
    pass_until(&mut engine, stack_is_empty);

    // Cast 1 card (-1), drew 2 cards for the two Birds on the battlefield (+2) -> net +1.
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}
