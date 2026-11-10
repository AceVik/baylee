//! `cards/lands/utility/mortuary_mire.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mortuary Mire: "This land enters tapped." / "When this land enters, you may put target creature card from your graveyard on top of your library." / "{T}: Add {B}."
/// Under `Coverage::Implemented`, playing Mortuary Mire enters tapped and triggers target selection.
/// Choosing a creature card in the graveyard moves it to the top of the library.
#[test]
fn mortuary_mire_enters_tapped_and_puts_creature_on_top_of_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(131, quiet_creature())
        .hand(0, &[mortuary_mire()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 1);
    let elf = in_graveyard(&engine, p0, quiet_creature()).expect("creature in graveyard");

    let land = play_land(&mut engine, p0, mortuary_mire());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&elf));

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .last()
            .copied(),
        Some(elf)
    );
    assert!(in_graveyard(&engine, p0, quiet_creature()).is_none());
}
