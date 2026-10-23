//! `cards/instants/mv_3/rebuild.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn rebuild() -> CardIndex {
    card_index("496999a7-a82a-45ea-9c4e-1b6aabc781cd")
}

fn thran_lens() -> CardIndex {
    card_index("0ac723bf-ce86-4028-a693-0ef498e4c82d")
}

#[test]
fn rebuild_bounces_all_artifacts() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), thran_lens()])
        .battlefield(1, &[thran_lens(), gray_ogre()])
        .hand(0, &[rebuild()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, rebuild());
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, thran_lens()).is_some());
    assert!(in_hand(&engine, p1, thran_lens()).is_some());
    assert!(
        on_battlefield(&engine, p1, gray_ogre()).is_some(),
        "nonartifacts stay"
    );
    assert_eq!(all_on_battlefield(&engine, p0, island()).len(), 3);
}

#[test]
fn cycling_discards_for_two_and_draws_one() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game(SEED, &[], &[], &[rebuild()], &[]);
    lk::float(&mut engine, lk::P0, &[(ManaColor::Colorless, 2)]);
    let before = engine.state().zones.list(ZoneLocation::Hand(lk::P0)).len();
    activate(&mut engine, lk::P0, rebuild(), 1);
    assert!(in_graveyard(&engine, lk::P0, rebuild()).is_some());
    assert_eq!(lk::pool_total(&engine, lk::P0), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(lk::P0)).len(),
        before
    );
}
