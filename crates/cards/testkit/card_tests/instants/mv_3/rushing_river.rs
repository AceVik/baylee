//! `cards/instants/mv_3/rushing_river.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "6e2a695d-305a-4bbe-b302-11fdc5c2a451"
fn rushing_river() -> CardIndex {
    card_index("6e2a695d-305a-4bbe-b302-11fdc5c2a451")
}

/// Rushing River ({2}{U}, Kicker—Sacrifice a land): unkicked excludes lands
/// and bounces one nonland permanent without sacrificing a land.
#[test]
fn rushing_river_unkicked_bounces_one_nonland_permanent() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[rushing_river()])
        .battlefield(1, &[plains(), sol_ring(), index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let land = lk::obj(&engine, P1, plains());
    let ring = lk::obj(&engine, P1, sol_ring());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, rushing_river());
    let normal = choose_cast_kind(&engine, CastModeKind::Normal);
    engine.apply(P0, PlayerAction::ChooseMode(normal)).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ring) && options.contains(&bears));
    assert!(
        !options.contains(&land),
        "lands are not legal targets: {options:?}"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![ring],
                players: vec![],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(all_on_battlefield(&engine, P0, island()).len(), 3);
    assert!(in_hand(&engine, P1, sol_ring()).is_some());
    assert_eq!(
        on_battlefield(&engine, P1, index::GRIZZLY_BEARS),
        Some(bears)
    );
}

/// Kicked by sacrificing a land: sacrifices one of your lands and returns TWO
/// target nonland permanents to their owners' hands.
#[test]
fn rushing_river_kicked_sacrifices_land_and_bounces_two_nonlands() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[rushing_river()])
        .battlefield(1, &[sol_ring(), index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let sac_island = all_on_battlefield(&engine, P0, island())[0];
    let ring = lk::obj(&engine, P1, sol_ring());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, rushing_river());
    let kicked = choose_cast_kind(&engine, CastModeKind::Kicked);
    engine.apply(P0, PlayerAction::ChooseMode(kicked)).unwrap();
    for _ in 0..6 {
        match engine.pending().clone() {
            Pending::ChooseTargets { min, max, .. } => {
                assert_eq!((min, max), (2, 2), "kicked chooses two distinct targets");
                engine
                    .apply(
                        P0,
                        PlayerAction::ChooseTargets {
                            objects: vec![ring, bears],
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCards { .. } => {
                engine
                    .apply(
                        P0,
                        PlayerAction::ChooseObjects {
                            objects: vec![sac_island],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    lk::settle(&mut engine);

    assert_eq!(all_on_battlefield(&engine, P0, island()).len(), 2);
    assert!(in_graveyard(&engine, P0, island()).is_some());
    assert!(in_hand(&engine, P1, sol_ring()).is_some());
    assert!(in_hand(&engine, P1, index::GRIZZLY_BEARS).is_some());
}
