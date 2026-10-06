//! `cards/sorceries/mv_3/maelstrom_pulse.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Maelstrom Pulse`: "Destroy target nonland permanent and all other
/// permanents with the same name as that permanent."
///
/// One of the opponent's two Llanowar Elves is the target. "All other
/// permanents" is everyone's, so the caster's own Elves goes too, and a
/// creature with another name stays. Lands are not targets.
#[test]
fn maelstrom_pulse_destroys_its_target_and_every_permanent_of_that_name() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(475, forest())
        .battlefield(0, &[swamp(), swamp(), forest(), llanowar_elves()])
        .battlefield(
            1,
            &[
                forest(),
                llanowar_elves(),
                llanowar_elves(),
                festering_goblin(),
            ],
        )
        .hand(0, &[maelstrom_pulse()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let their_land = on_battlefield(&engine, p1, forest()).expect("opponent land");
    let their_elves = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(their_elves.len(), 2, "opponent has two elves");
    let my_elves = on_battlefield(&engine, p0, llanowar_elves()).expect("p0's own Elves");
    let goblin = on_battlefield(&engine, p1, festering_goblin()).expect("the Goblin");

    cast_from_hand(&mut engine, p0, maelstrom_pulse());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target prompt, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&their_elves[0]),
        "nonland permanent is a legal target"
    );
    assert!(
        !options.contains(&their_land),
        "lands are not legal targets for Maelstrom Pulse"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_elves[0]],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    // By id: a destroyed permanent is still an object, in its owner's
    // graveyard, so "is it an object" says nothing — "is it on the
    // battlefield" does.
    let battlefield = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .clone();
    assert!(
        !battlefield.contains(&their_elves[0]),
        "the targeted creature was destroyed"
    );
    assert!(
        !battlefield.contains(&their_elves[1]),
        "the opponent's other Llanowar Elves shares its name"
    );
    assert!(
        !battlefield.contains(&my_elves),
        "and so does the caster's own: \"all other permanents\" is everyone's"
    );
    assert!(
        battlefield.contains(&goblin),
        "a permanent with another name stays"
    );
    assert!(
        in_graveyard(&engine, p0, maelstrom_pulse()).is_some(),
        "Maelstrom Pulse moved to graveyard after resolving"
    );
}
