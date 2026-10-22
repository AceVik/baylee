//! `cards/artifacts/mv_2/wand_of_denial.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

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
