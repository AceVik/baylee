//! `cards/creatures/mv_1/child_of_thorns.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Child of Thorns prints one line: "Sacrifice this creature: Target creature
/// gets +1/+1 until end of turn." The interesting half of that sentence is the
/// order of its two halves — CR 601.2c names the target before CR 601.2h
/// sacrifices the source, so while the target question stands the 1/1 Spirit
/// is still on the battlefield paying for nothing yet, and the target's own
/// pumped body is only readable once the ability resolves. "Target creature"
/// is also the word that reaches across the table, so the seat that gets the
/// +1/+1 is the *opponent's* Elf, with an untargeted Elf under the Spirit's
/// own controller beside it as the control for "the creature it named and no
/// other".
#[test]
fn child_of_thorns_sacrifices_itself_to_pump_the_creature_it_targets() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), llanowar_elves()])
        .hand(0, &[child_of_thorns()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The Spirit arrives the way the card arrives: {G} off the one Forest.
    cast_from_hand(&mut engine, p0, child_of_thorns());
    pass_until(&mut engine, stack_is_empty);
    let child = on_battlefield(&engine, p0, child_of_thorns()).expect("the Spirit resolved");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, child), (1, 1), "a printed 1/1");
    assert_eq!(pt(&engine, theirs), (1, 1), "and so is the Elf it is about");

    // The one line the Spirit prints asks nothing of the pool — its whole
    // price is the body — so it is offered without a mana floating anywhere.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(child, 0)),
        "the sacrifice-to-pump line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, child_of_thorns(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("the pump targets a creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat names the target");
    assert!(
        options.contains(&child) && options.contains(&mine),
        "\"target creature\" is any creature this side of the table: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "and it reaches across the table: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, child_of_thorns()).is_some(),
        "CR 601.2c before CR 601.2h: the target is named while the Spirit \
         that will pay for it is still on the battlefield"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Elf was one of the options");

    assert!(
        on_battlefield(&engine, p0, child_of_thorns()).is_none(),
        "the Spirit was sacrificed as the cost of its own ability"
    );
    assert!(
        in_graveyard(&engine, p0, child_of_thorns()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "no mana ability, so the pump is waiting on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, theirs),
        (2, 2),
        "+1/+1 until end of turn on the creature that was targeted"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "the Elf nobody named is untouched, so the pump is one target and \
         not every creature on the board"
    );
}
