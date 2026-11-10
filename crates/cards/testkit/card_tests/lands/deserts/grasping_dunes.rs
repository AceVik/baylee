//! `cards/lands/deserts/grasping_dunes.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Grasping Dunes: "{T}: Add {C}." / "{1}, {T}, Sacrifice this land: Put a -1/-1 counter on target creature. Activate only as a sorcery."
/// Paid with a Forest for {1}, the activated ability targets Llanowar Elves and sacrifices Grasping Dunes as a cost.
/// Upon resolution, the -1/-1 counter reduces the 1/1 creature to 0 toughness, sending it to the graveyard.
#[test]
fn grasping_dunes_places_minus_counter_and_destroys_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(114, forest())
        .battlefield(0, &[grasping_dunes(), forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let dunes = on_battlefield(&engine, p0, grasping_dunes()).expect("Dunes deployed");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("Elves deployed");

    tap_mana_except(&mut engine, p0, dunes);
    activate(&mut engine, p0, grasping_dunes(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&elves),
        "Llanowar Elves is a valid creature target"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
    assert!(in_graveyard(&engine, p0, grasping_dunes()).is_some());
}
