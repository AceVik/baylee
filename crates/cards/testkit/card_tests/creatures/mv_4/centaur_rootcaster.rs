//! `cards/creatures/mv_4/centaur_rootcaster.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn centaur_rootcaster_combat_damage_to_player_searches_for_a_basic_land_tapped() {
    let rootcaster = card_index("0aaefa00-86f4-4e12-87ae-3a362d862fa4");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[rootcaster])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let centaur = on_battlefield(&engine, p0, rootcaster).expect("Centaur Rootcaster on board");
    let forests_before = all_on_battlefield(&engine, p0, forest()).len();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(centaur, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected basic land search choice");
    };
    let chosen_forest = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen_forest],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let forests_after = all_on_battlefield(&engine, p0, forest());
    assert_eq!(forests_after.len(), forests_before + 1);
    assert!(
        is_tapped(&engine, chosen_forest),
        "searched basic land enters tapped"
    );
}
