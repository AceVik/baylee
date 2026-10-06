//! `cards/creatures/mv_5/fallen_angel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fallen Angel is a {3}{B}{B} 3/3 flier whose printed text beyond the keyword
/// is one activated line: "Sacrifice a creature: This creature gets +2/+1 until
/// end of turn."
///
/// Three words of that sentence each need their own witness on the board, and
/// all three are read in one game: "a creature" is a sacrifice menu holding the
/// two Elves this seat controls — the Angel included, since it is one too — and
/// neither the opponent's Elf nor the Swamps (CR 701.21a); "This creature"
/// leaves the Elf nobody gave up at a printed 1/1 while the Angel becomes a
/// 5/4; and "until end of turn" is gone once a turn has really passed. The five
/// Swamps pay {3}{B}{B} out of the pool, so the cast is played and not assumed.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn fallen_angel_sacrifices_a_creature_of_its_own_side_to_grow_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[fallen_angel()])
        // The same 1/1 across the table: "a creature" is read as the seat's
        // own, and a same-card bystander is the only thing that can say so.
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which is the offering");
    let (fodder, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // Five Swamps pay {3}{B}{B}; the Elves are named as the printing kept back,
    // because one of them is the creature this ability is about to eat and both
    // are creatures this test reads back afterwards.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Swamps tapped and neither Elf: five black, which is exactly \
         {{3}}{{B}}{{B}}"
    );
    cast_with_floating(&mut engine, p0, fallen_angel());
    pass_until(&mut engine, stack_is_empty);

    let angel = on_battlefield(&engine, p0, fallen_angel()).expect("the Angel resolved");
    assert_eq!(pt(&engine, angel), (3, 3), "the body the card prints");
    assert!(
        keywords(&engine, angel).contains(KeywordSet::FLYING),
        "and the printed flying reaches the permanent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}}{{B}}{{B}} is spent, so nothing floating can be mistaken for \
         a price below"
    );

    // The ability's only price is a creature, so no mana has to be floating for
    // it to be offered at all — which is what makes the menu the reading.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(angel, 0)),
        "a creature to eat and no mana to pay: the one line the card prints is \
         offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, fallen_angel(), 0);
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
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostSacrifice,
        "the variant is what tells a client this is a cost and not a search"
    );
    assert_eq!((min, max), (1, 1), "one creature, and the cost asks once");
    assert_eq!(
        options.len(),
        3,
        "the three creatures this seat controls, and the five Swamps are lands: \
         {options:?}"
    );
    assert!(
        options.contains(&fodder) && options.contains(&bystander),
        "both Elves are creatures you control: {options:?}"
    );
    assert!(
        options.contains(&angel),
        "and so is the Angel itself, which is a creature you control like any \
         other (CR 701.21a): {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "an opponent's creature is not yours to sacrifice: {options:?}"
    );

    let refused = engine.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![theirs],
        },
    );
    assert!(
        refused.is_err(),
        "an answer the question did not enumerate is refused: {refused:?}"
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
        "a sacrificed creature goes to its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so the ability is on the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, angel),
        (5, 4),
        "+2/+1 on the creature the ability belongs to — a (5, 5) would mean a \
         toughness the card does not print"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody gave up is untouched: the pump names `This` and not \
         every creature on the board"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and it never reaches across the table"
    );
    assert!(
        on_battlefield(&engine, p0, fallen_angel()).is_some(),
        "the Angel outlives the creature it ate"
    );

    // "until end of turn": a turn later the Angel is the 3/3 it was printed as,
    // which no reading taken at a single moment can see.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, angel),
        (3, 3),
        "the grant lasted the turn it was made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, fallen_angel()).is_some(),
        "and the Angel is still standing, so the pump left rather than the creature"
    );
}
