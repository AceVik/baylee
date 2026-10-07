//! `cards/creatures/mv_3/nantuko_husk.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nantuko Husk is a {2}{B} 2/2 whose only printed line is "Sacrifice a
/// creature: This creature gets +2/+2 until end of turn." The price names no
/// creature in particular, so the engine has to ask which one — and that menu
/// is half the card: both creatures this seat controls are on it (the Husk is
/// one of them, CR 701.16a) while CR 701.21a keeps the opponent's Elf off it.
/// Sacrificing is the last step of the activation (CR 601.2h), so the fodder
/// is already buried while the +2/+2 is still on the stack, which is the only
/// way one board can tell the printed 2/2 from the pumped 4/4.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn nantuko_husk_eats_a_creature_you_control_to_grow_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[nantuko_husk()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    cast_from_hand(&mut engine, p0, nantuko_husk());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let husk = on_battlefield(&engine, p0, nantuko_husk()).expect("the Husk resolved");
    let fodder = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    assert_eq!(pt(&engine, husk), (2, 2), "the body the card prints");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(husk, 0)),
        "the one line the Husk prints costs a creature and no mana at all, so \
         it is offered without a single mana floating: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, nantuko_husk(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the cost asks which creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one asked");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert_eq!(
        options.len(),
        2,
        "the two creatures this seat controls and nothing else: {options:?}"
    );
    assert!(
        options.contains(&husk),
        "the Husk is itself a creature you control, so it is on its own menu: {options:?}"
    );
    assert!(
        options.contains(&fodder),
        "and the Elf beside it is as much a creature as the Husk: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "`CR 701.21a`: an opponent's creature is not yours to sacrifice: {options:?}"
    );

    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![theirs],
                },
            )
            .is_err(),
        "an answer the question did not enumerate is refused"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the creature the question offered pays the cost");

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "CR 601.2h: the creature is sacrificed as the cost, before the ability \
         is on the stack"
    );
    assert!(
        !stack_is_empty(&engine),
        "and the ability is what is waiting: it is no mana ability"
    );
    assert_eq!(
        pt(&engine, husk),
        (2, 2),
        "nothing is pumped until the ability resolves"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, husk),
        (4, 4),
        "+2/+2 until end of turn, on the permanent that paid the price"
    );
    assert!(
        on_battlefield(&engine, p0, nantuko_husk()).is_some(),
        "the Husk ate the Elf and not itself, so it is still standing"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the opponent's board never moved"
    );
}
