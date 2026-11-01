//! `cards/artifacts/mv_2/wand_of_denial.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn wand_of_denial() -> CardIndex {
    card_index("9d32da71-60e5-4eb3-97ca-ae76e2f5df57")
}

/// `Wand of Denial` ({2}, Artifact, `Coverage::Partial`) taps to look at the
/// top card of target player's library.
#[test]
fn wand_of_denial_taps_to_look_at_top_card_of_target_players_library() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[wand_of_denial()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, wand_of_denial());
    pass_until(&mut engine, stack_is_empty);
    let wand = on_battlefield(&engine, p0, wand_of_denial()).unwrap();

    activate(&mut engine, p0, wand_of_denial(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards LookAtLibrary");
    };
    assert_eq!(options.len(), 1);
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(wand)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
}

/// The look is all the card does as written: the card shown is the *target
/// player's* top card, nothing leaves either library, no life is paid, and the
/// {T} cost keeps a second look out of the same turn.
#[test]
fn wand_of_denial_only_looks_at_the_targets_top_card_and_stays_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[wand_of_denial()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let wand = on_battlefield(&engine, p0, wand_of_denial()).unwrap();
    let (mine, theirs) = (library_size(&engine, p0), library_size(&engine, p1));

    assert!(lk::offered(&engine, wand_of_denial(), p0, 0));
    activate(&mut engine, p0, wand_of_denial(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseCards LookAtLibrary");
    };
    assert_eq!(player, p0, "the Wand's controller looks");
    assert_eq!(options.len(), 1);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p1))
            .contains(&options[0]),
        "the card shown is the targeted player's, not the controller's"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(library_size(&engine, p0), mine, "looking takes nothing");
    assert_eq!(library_size(&engine, p1), theirs, "and mills nothing");
    assert_eq!(lk::life(&engine, p0), 20, "no 2 life is paid as written");
    assert_eq!(lk::life(&engine, p1), 20);
    assert!(lk::tapped(&engine, wand));
    assert!(
        !lk::offered(&engine, wand_of_denial(), p0, 0),
        "tapped, the Wand cannot look a second time"
    );
}
