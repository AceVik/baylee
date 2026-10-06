//! `cards/instants/mv_4/fell_the_profane.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fell the Profane // Fell Mire (`Coverage::Implemented`): "Destroy target
/// creature or planeswalker. // As this land enters, you may pay 3 life. If
/// you don't, it enters tapped. {T}: Add {B}."
///
/// The front face destroys a creature or planeswalker. The test casts Fell the
/// Profane targeting an opponent's Llanowar Elves, verifies the target is
/// destroyed upon resolution, and checks that both the destroyed creature and
/// the spell card arrive in their owners' graveyards.
#[test]
fn fell_the_profane_destroys_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(48, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[fell_the_profane()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent's Elf");

    cast_from_hand(&mut engine, p0, fell_the_profane());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&elf),
        "target creature or planeswalker — the Elf qualifies"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal target");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the targeted creature is no longer on the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the destroyed creature is in its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, fell_the_profane()).is_some(),
        "the resolved spell is in its caster's graveyard"
    );
}
