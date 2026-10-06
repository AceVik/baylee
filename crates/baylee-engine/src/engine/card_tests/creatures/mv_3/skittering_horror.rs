//! `cards/creatures/mv_3/skittering_horror.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Skittering Horror is a {2}{B} 4/3 with one printed sentence: "When you
/// cast a creature spell, sacrifice this creature." Every word of it is read
/// on one board, because each one is a chance for the trigger to be the wrong
/// trigger: its own cast leaves it standing (a permanent's ability does
/// nothing from a stack), Dark Ritual leaves it standing (an instant is no
/// creature spell), the opponent's Goblin leaves it standing ("you" is the
/// Horror's controller), and the controller's own Goblin takes it away a step
/// *before* that Goblin resolves — which is the difference between a trigger
/// on the casting and one on the entry.
#[test]
fn skittering_horror_sacrifices_itself_for_its_controllers_creature_spell_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[skittering_horror(), dark_ritual(), festering_goblin()])
        .battlefield(1, &[swamp()])
        .hand(1, &[festering_goblin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Its own cast: the trigger belongs to a permanent, so the spell that is
    // still a card on the stack raises nothing and the Horror arrives.
    cast_from_hand(&mut engine, p0, skittering_horror());
    pass_until(&mut engine, stack_is_empty);
    let horror = on_battlefield(&engine, p0, skittering_horror())
        .expect("the Horror resolved, and its own cast did not sacrifice it");
    assert_eq!(
        pt(&engine, horror),
        (4, 3),
        "the printed body, on the table"
    );

    // A spell that is no creature: `YOUR_CREATURE` reads the card type, and
    // Dark Ritual costs the one black the Horror left in the pool.
    cast_from_hand(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, skittering_horror()).is_some(),
        "an instant is no creature spell, so the Horror stays where it is"
    );

    // And a creature spell across the table: "you" is whoever controls the
    // Horror, never whoever happens to be casting.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, festering_goblin());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p1, festering_goblin()).is_some(),
        "the Goblin resolved for the seat that cast it"
    );
    assert!(
        on_battlefield(&engine, p0, skittering_horror()).is_some(),
        "another seat's creature spell is not this Horror's to answer"
    );

    // The real thing, back on p0's turn with the Swamps untapped.
    reach_their_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, festering_goblin());
    pass_until(&mut engine, |e| {
        in_graveyard(e, p0, skittering_horror()).is_some()
    });
    assert!(
        on_battlefield(&engine, p0, festering_goblin()).is_none() && !stack_is_empty(&engine),
        "the trigger resolves above the spell that raised it, so the Goblin \
         is still on the stack while the Horror is already gone — a trigger \
         on the creature *entering* would have it the other way round"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, festering_goblin()).is_some(),
        "and then the creature spell the trigger was about resolves"
    );
}
