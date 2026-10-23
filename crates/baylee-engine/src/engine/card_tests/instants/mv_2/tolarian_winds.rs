#[allow(clippy::wildcard_imports)]
use super::*;

fn tolarian_winds() -> CardIndex {
    card_index("686cce6e-18ec-45b0-8d2e-74fa353a905e")
}

#[test]
fn tolarian_winds_discards_hand_and_draws_that_many() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[tolarian_winds(), gray_ogre(), wild_elephant()])
        .battlefield(0, &[island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    cast_from_hand(&mut engine, p0, tolarian_winds());
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, gray_ogre()).is_some());
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        hand_before - 1
    );
}
