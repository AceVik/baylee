//! `cards/creatures/mv_3/whip_sergeant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Whip Sergeant is a {2}{R} 2/1 whose entire printed text is "{R}: Target
/// creature gains haste until end of turn." Four Mountains cast it and a
/// Forest beside them pays for a Llanowar Elves, so the board holds a
/// creature that arrived this turn — summoning sick, which is the only thing
/// the ability is for — while one red left floating pays for the pump. Haste
/// is then read where it is worth something: the Elf that was named is in the
/// attack declaration the turn it arrived, and the Sergeant, which was never
/// named, is not. The opponent's Elf stands on the target menu as the half
/// that says "target creature" and not "target creature you control".
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn whip_sergeant_spends_red_to_let_a_creature_attack_the_turn_it_arrived() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), forest()],
        )
        .hand(0, &[whip_sergeant(), llanowar_elves()])
        // A creature across the table, so the target filter is read: the card
        // says "target creature" and never "you control".
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The Forest is kept back: it pays for the Elf, and the four Mountains pay
    // the {2}{R} with one red to spare for the ability itself.
    tap_all_mana_but(&mut engine, p0, Some(forest()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Mountains, four red"
    );
    cast_with_floating(&mut engine, p0, whip_sergeant());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let sergeant = on_battlefield(&engine, p0, whip_sergeant()).expect("the Sergeant resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "{{2}}{{R}} out of four red leaves exactly one"
    );

    // The creature the ability is about: it arrives this turn and is summoning
    // sick, which is the whole reason the keyword is worth anything.
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves resolved");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, elves), (1, 1), "a printed 1/1 before the pump");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the Forest's green paid for the Elf, so only the Sergeant's leftover \
         red is floating"
    );
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::HASTE),
        "and it is summoning sick, so it may not attack yet"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(sergeant, 0)),
        "with one red floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, whip_sergeant(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&elves) && options.contains(&theirs) && options.contains(&sergeant),
        "\"target creature\" is any creature on either side of the table, and \
         the Sergeant may point it at itself: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "CR 601.2h pays last: the {{R}} is still in the pool while the target \
         question stands"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the Elves were one of the options it enumerated");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{R}} came out of the pool"
    );
    assert!(
        !is_tapped(&engine, sergeant),
        "the printed cost is {{R}} alone, so nothing tapped to pay it"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        keywords(&engine, elves).contains(KeywordSet::HASTE),
        "\"target creature gains haste until end of turn\""
    );
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "the pump is 0/0: the ability grants the keyword and no body"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::HASTE),
        "the effect reaches the creature that was named and never across the table"
    );
    assert!(
        !keywords(&engine, sergeant).contains(KeywordSet::HASTE),
        "nor its own source, which was not the target"
    );

    // The point of the keyword, read off the attack declaration instead of off
    // a keyword set: the hasted Elf may attack the turn it arrived, and the
    // Sergeant — untapped, but never named — is still summoning sick.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(
        attackers.contains(&elves),
        "haste is what lets a creature attack the turn it arrived (CR 302.6): \
         {attackers:?}"
    );
    assert!(
        !attackers.contains(&sergeant),
        "and the control is the creature the ability never named: a 2/1 that \
         entered this turn is not offered: {attackers:?}"
    );
}
