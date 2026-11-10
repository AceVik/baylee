//! `cards/sorceries/mv_3/undo.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn undo() -> CardIndex {
    card_index("b446c3bd-3c7d-4d59-ba59-3dd80818465e")
}

/// `Undo` ({1}{U}{U}, Sorcery, `Coverage::Implemented`) returns two target
/// creatures to their owners' hands.
#[test]
fn undo_returns_two_target_creatures_to_their_owners_hands() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), GRIZZLY_BEARS])
        .battlefield(1, &[HILL_GIANT, llanowar_elves()])
        .hand(0, &[undo()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let c1 = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let c2 = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
    let c3 = on_battlefield(&engine, p1, llanowar_elves()).unwrap();

    cast_from_hand(&mut engine, p0, undo());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![c1, c2],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, GRIZZLY_BEARS).is_some());
    assert!(in_hand(&engine, p1, HILL_GIANT).is_some());
    assert_eq!(engine.state().object(c3).unwrap().zone, Zone::Battlefield);
}

/// "Two target creatures" is exactly two: one target does not cast the spell,
/// and the question stays open for a legal answer.
#[test]
fn undo_refuses_a_single_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), GRIZZLY_BEARS])
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[undo()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, p0, undo());
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![giant],
                    players: vec![],
                },
            )
            .is_err(),
        "one target is not two"
    );
    assert!(matches!(engine.pending(), Pending::ChooseTargets { .. }));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears, giant],
                players: vec![],
            },
        )
        .expect("two targets are legal");
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p0, GRIZZLY_BEARS).is_some());
    assert!(in_hand(&engine, p1, HILL_GIANT).is_some());
}
