#[allow(clippy::wildcard_imports)]
use super::*;

fn turnabout() -> CardIndex {
    card_index("715d0777-4d36-463b-9102-6bb6bb00b7e9")
}

#[test]
fn turnabout_taps_all_creatures_target_player_controls() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[turnabout()])
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[gray_ogre(), wild_elephant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("ogre on bf");
    let elephant = on_battlefield(&engine, p1, wild_elephant()).expect("elephant on bf");
    cast_from_hand(&mut engine, p0, turnabout());
    match engine.pending().clone() {
        Pending::ChooseTargets { .. } => {
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![p1],
                    },
                )
                .unwrap();
        }
        Pending::ChoosePlayer { .. } => {
            engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
        }
        other => panic!("unexpected pending: {other:?}"),
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(ogre)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert!(
        engine
            .state()
            .object(elephant)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
}

#[test]
fn turnabout_partial_creature_tap_excludes_other_players_and_noncreatures() {
    use crate::engine::card_tests::legends_kit as lk;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[gray_ogre()],
        &[grizzly_bears(), forest(), sol_ring()],
        &[turnabout()],
        &[],
    );
    let mine = lk::obj(&e, p0, gray_ogre());
    let theirs = lk::obj(&e, p1, grizzly_bears());
    let land = lk::obj(&e, p1, forest());
    let artifact = lk::obj(&e, p1, sol_ring());
    lk::float(&mut e, p0, &[(ManaColor::Blue, 4)]);
    cast_with_floating(&mut e, p0, turnabout());
    match e.pending() {
        Pending::ChooseTargets { .. } => lk::aim(&mut e, &[], &[p1]),
        Pending::ChoosePlayer { .. } => {
            e.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
        }
        other => panic!("player: {other:?}"),
    }
    lk::settle(&mut e);
    assert!(is_tapped(&e, theirs));
    for id in [mine, land, artifact] {
        assert!(!is_tapped(&e, id));
    }
}
