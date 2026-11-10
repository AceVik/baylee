//! `cards/creatures/mv_5/deadly_insect.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "98273b0d-2b41-486b-9498-30a692c03982"

/// Deadly Insect is a {4}{G} 6/1 whose entire printed text is Shroud — "This
/// creature can't be the target of spells or abilities." Five Forests pay for
/// it exactly, and the Llanowar Elves of the same seat is the creature the
/// opponent's Swords to Plowshares *is* offered, so an offer without the
/// Insect is the shroud and not an empty target menu; the removal then
/// resolving onto that Elf while the Insect keeps standing is the control the
/// offer alone cannot give.
#[test]
fn deadly_insect_shrouds_itself_from_the_opponents_removal_where_a_plain_creature_does_not() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[deadly_insect()])
        .battlefield(1, &[plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    // Five Forests pay {4}{G}, and the Elf is named as the printing kept back:
    // it is the creature the opponent's removal is about to be offered, and a
    // mana creature tapped for the Insect would have left the target menu
    // empty for a reason that has nothing to do with shroud (rule 11).
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Forests tapped, and neither the Elf nor a sixth source paid in"
    );
    cast_with_floating(&mut engine, p0, deadly_insect());
    pass_until(&mut engine, stack_is_empty);

    let insect = on_battlefield(&engine, p0, deadly_insect()).expect("the Insect resolved");
    assert_eq!(pt(&engine, insect), (6, 1), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{4}}{{G}} spent the whole pool"
    );

    // Across to the opponent, whose Swords is the removal this card has to
    // shrug off.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, swords_to_plowshares());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this question")
    };
    assert_eq!(player, p1, "the seat casting the Swords names the target");
    assert_eq!((min, max), (1, 1), "one creature, and the spell asks once");
    assert!(
        options.contains(&elf),
        "a creature of mine with no shroud is a legal target: {options:?}"
    );
    assert!(
        !options.contains(&insect),
        "\"This creature can't be the target of spells or abilities\" — the \
         Insect is on the battlefield and is not one of the offers: {options:?}"
    );

    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf was one of the options the question enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the Swords resolved against the only creature it was allowed to name"
    );
    assert!(
        on_battlefield(&engine, p0, deadly_insect()).is_some(),
        "and the shrouded Insect was never a candidate for it"
    );
}
