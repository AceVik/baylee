//! `cards/instants/mv_2/break_open.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;
use baylee_engine::object::Status;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "1fcec3b2-47ba-4c56-9ce1-a4807e8ad262"
fn break_open() -> CardIndex {
    card_index("1fcec3b2-47ba-4c56-9ce1-a4807e8ad262")
}

/// Break Open ({1}{R}): targets ONLY a face-down creature an opponent controls
/// (excluding face-up opponent creatures and your own face-down creatures) and
/// turns it face up.
#[test]
fn break_open_turns_only_opponents_face_down_creature_face_up() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), forest(), forest(), forest()])
        .hand(0, &[index::NANTUKO_VIGILANTE, break_open()])
        .battlefield(
            1,
            &[mountain(), mountain(), mountain(), index::GRIZZLY_BEARS],
        )
        .hand(1, &[index::NANTUKO_VIGILANTE])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // P0 casts their own Nantuko Vigilante face down for {3}.
    let p0_forests = all_on_battlefield(&engine, P0, forest());
    tap_mana_where(&mut engine, P0, |id| p0_forests.contains(&id));
    cast_with_floating(&mut engine, P0, index::NANTUKO_VIGILANTE);
    lk::settle(&mut engine);
    let my_morph = lk::obj(&engine, P0, index::NANTUKO_VIGILANTE);
    assert!(
        engine
            .state()
            .object(my_morph)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );

    // P1 casts their Nantuko Vigilante face down for {3} on P1's turn.
    reach_their_main_phase(&mut engine, P1);
    tap_all_mana(&mut engine, P1);
    cast_with_floating(&mut engine, P1, index::NANTUKO_VIGILANTE);
    pass_until(&mut engine, stack_is_empty);
    let their_morph = lk::obj(&engine, P1, index::NANTUKO_VIGILANTE);
    let their_bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    assert!(
        engine
            .state()
            .object(their_morph)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert_eq!(pt(&engine, their_morph), (2, 2));

    // P1 passes priority -> P0 casts Break Open targeting P1's face-down creature.
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    let p0_mtns = all_on_battlefield(&engine, P0, mountain());
    tap_mana_where(&mut engine, P0, |id| p0_mtns.contains(&id));
    cast_with_floating(&mut engine, P0, break_open());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert_eq!(
        options,
        vec![their_morph],
        "only opponent's face-down creature is a legal target (excludes {their_bears:?} and {my_morph:?})"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![their_morph],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !engine
            .state()
            .object(their_morph)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN),
        "opponent's morph was turned face up"
    );
    assert_eq!(
        pt(&engine, their_morph),
        (3, 2),
        "Nantuko Vigilante is 3/2 face up"
    );
    assert!(
        engine
            .state()
            .object(my_morph)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN),
        "P0's own face-down creature stayed face down"
    );
}
