//! `cards/creatures/artifacts/mv_4/rustspore_ram.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Rustspore Ram` prints `When this creature enters, destroy target Equipment.`
/// with `Coverage::Implemented`.
/// Cast from hand off four `forest()` lands, `Rustspore Ram` enters the battlefield and its
/// enters trigger goes onto the stack, prompting for an Equipment target.
/// Targeting the opponent's `lightning_greaves()` and resolving the trigger destroys the Equipment,
/// sending it to the opponent's graveyard while the ram remains on the battlefield.
#[test]
fn rustspore_ram_destroys_target_equipment_on_entering() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[rustspore_ram()])
        .battlefield(1, &[lightning_greaves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let greaves = on_battlefield(&engine, p1, lightning_greaves()).expect("equipment seated");

    cast_from_hand(&mut engine, p0, rustspore_ram());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on ChooseTargets");
    };
    assert!(options.contains(&greaves));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![greaves],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, lightning_greaves()).is_none(),
        "the equipment was destroyed"
    );
    assert!(
        in_graveyard(&engine, p1, lightning_greaves()).is_some(),
        "the destroyed equipment is in the opponent's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, rustspore_ram()).is_some(),
        "`Rustspore Ram` remains on the battlefield"
    );
}
