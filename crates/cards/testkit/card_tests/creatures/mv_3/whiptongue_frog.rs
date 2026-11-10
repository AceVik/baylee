//! `cards/creatures/mv_3/whiptongue_frog.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Whiptongue Frog is a {2}{U} 1/3 whose whole printed text is one line:
/// "{U}: This creature gains flying until end of turn." The pump names
/// `Filter::This`, so the claim to play is that the keyword lands on the Frog
/// and on nothing else — which is why an untapped Llanowar Elves stands beside
/// it as the control, kept out of the mana by `tap_all_mana_but` and still
/// grounded when the Frog flies. Four Islands pay the {3} and leave exactly the
/// {U} the activation charges, so the pool is a real payment rather than a
/// label on a free ability, and a full turn cycle afterwards is what reads the
/// duration instead of a keyword the creature simply has.
#[test]
fn whiptongue_frog_pays_blue_to_fly_and_lands_when_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), llanowar_elves()],
        )
        .hand(0, &[whiptongue_frog()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");

    // Four Islands and only those: the Elf is the creature this test reads back
    // afterwards, and tapping it for mana would take it out of the board for a
    // reason of its own.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Islands, and the Elf paid nothing"
    );
    cast_with_floating(&mut engine, p0, whiptongue_frog());
    pass_until(&mut engine, stack_is_empty);

    let frog = on_battlefield(&engine, p0, whiptongue_frog()).expect("the Frog resolved");
    assert_eq!(pt(&engine, frog), (1, 3), "the printed 1/3 body");
    assert!(
        !keywords(&engine, frog).contains(KeywordSet::FLYING),
        "a creature with no flying on its printed line starts grounded"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "{{2}}{{U}} was spent and the activation's {{U}} is still floating"
    );

    // Ability 0 is the only line the card prints, and its price is mana and no
    // tap: the offer is read with the Frog still untapped.
    activate(&mut engine, p0, whiptongue_frog(), 0);
    assert!(
        !is_tapped(&engine, frog),
        "{{U}} alone is the whole price, so nothing about the creature moves"
    );
    assert!(
        !stack_is_empty(&engine),
        "granting a keyword is no mana ability, so the ability is on the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{U}} came out of the pool"
    );
    assert_eq!(
        pt(&engine, frog),
        (1, 3),
        "the pump is +0/+0: the grant is the keyword and not a body"
    );
    assert!(
        keywords(&engine, frog).contains(KeywordSet::FLYING),
        "\"{{U}}: This creature gains flying until end of turn\""
    );
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::FLYING),
        "the pump names `Filter::This`: the Elf beside it stays grounded"
    );

    // Across the opponent's turn and back, which is where "until end of turn"
    // has to have worn off in the cleanup step of the turn it was made in.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !keywords(&engine, frog).contains(KeywordSet::FLYING),
        "the grant expired with the turn, so the Frog is grounded again on its \
         controller's next main phase"
    );
}
