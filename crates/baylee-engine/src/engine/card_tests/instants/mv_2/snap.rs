//! `cards/instants/mv_2/snap.rs`, played.

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

#[test]
fn snap_untap_is_optional_and_can_choose_an_opponents_land() {
    use crate::engine::card_tests::legends_kit as lk;
    for pick in [false, true] {
        let mut engine = lk::game(SEED, &[forest()], &[island(), gray_ogre()], &[snap()], &[]);
        let mine = lk::obj(&engine, lk::P0, forest());
        let theirs = lk::obj(&engine, lk::P1, island());
        let ogre = lk::obj(&engine, lk::P1, gray_ogre());
        for land in [mine, theirs] {
            engine
                .dev_state_mut(lk::P0)
                .unwrap()
                .object_mut(land)
                .unwrap()
                .status
                .insert(Status::TAPPED);
        }
        lk::float(&mut engine, lk::P0, &[(ManaColor::Blue, 2)]);
        cast_with_floating(&mut engine, lk::P0, snap());
        let Pending::ChooseTargets { options, .. } = engine.pending() else {
            panic!("creature target")
        };
        assert!(options.contains(&ogre));
        assert!(!options.contains(&mine));
        lk::aim(&mut engine, &[ogre], &[]);
        let Pending::ChooseCards {
            options, min, max, ..
        } = pass_to_card_choice(&mut engine)
        else {
            unreachable!()
        };
        assert_eq!((min, max), (0, 2));
        assert!(options.contains(&mine) && options.contains(&theirs));
        engine
            .apply(
                lk::P0,
                PlayerAction::ChooseObjects {
                    objects: if pick { vec![theirs] } else { vec![] },
                },
            )
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert!(is_tapped(&engine, mine));
        assert_eq!(is_tapped(&engine, theirs), !pick);
        assert!(in_hand(&engine, lk::P1, gray_ogre()).is_some());
    }
}
