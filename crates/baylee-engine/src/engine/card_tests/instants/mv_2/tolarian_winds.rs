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
        .hand(1, &[gray_ogre()])
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
    assert!(in_hand(&engine, PlayerId::new(1), gray_ogre()).is_some());
    assert!(in_graveyard(&engine, p0, wild_elephant()).is_some());
}

#[test]
fn tolarian_winds_with_no_other_cards_draws_nothing() {
    use crate::engine::card_tests::legends_kit as lk;
    let p0 = PlayerId::new(0);
    let mut e = lk::game(SEED, &[], &[], &[tolarian_winds()], &[]);
    lk::float(&mut e, p0, &[(ManaColor::Blue, 2)]);
    lk::cast_at(&mut e, p0, tolarian_winds(), &[], &[]);
    assert!(
        e.state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .is_empty()
    );
}
