#[allow(clippy::wildcard_imports)]
use super::*;

fn snap() -> CardIndex {
    card_index("ac914d98-221e-426c-8a50-342896b15f9e")
}

#[test]
fn snap_bounces_creature_and_untaps_two_lands() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[snap()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    cast_from_hand(&mut engine, p0, snap());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![ogre],
                    players: vec![],
                },
            )
            .unwrap();
    }
    let Pending::ChooseCards { options, .. } = pass_to_card_choice(&mut engine) else {
        panic!("expected ChooseCards for Snap untap");
    };
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: options })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p1, gray_ogre()).is_some());
    for land in all_on_battlefield(&engine, p0, island()) {
        assert!(!is_tapped(&engine, land));
    }
}
