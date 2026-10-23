//! `cards/sorceries/mv_2/flame_jet.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn flame_jet() -> CardIndex {
    card_index("8a9d6a30-e2c2-4da5-ad3b-d64fb2b5543c")
}

#[test]
fn flame_jet_deals_three_damage_to_target_player() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[flame_jet()])
        .battlefield(0, &[mountain(), mountain(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, flame_jet());
    let creature = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("damage targets")
    };
    assert!(!options.contains(&creature));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 17);
}

#[test]
fn cycling_discards_for_two_and_draws_one() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game(SEED, &[], &[], &[flame_jet()], &[]);
    lk::float(&mut engine, lk::P0, &[(ManaColor::Colorless, 2)]);
    let before = engine.state().zones.list(ZoneLocation::Hand(lk::P0)).len();
    activate(&mut engine, lk::P0, flame_jet(), 1);
    assert!(in_graveyard(&engine, lk::P0, flame_jet()).is_some());
    assert_eq!(lk::pool_total(&engine, lk::P0), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(lk::P0)).len(),
        before
    );
}
