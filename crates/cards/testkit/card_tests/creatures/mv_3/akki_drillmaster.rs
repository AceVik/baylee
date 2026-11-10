//! `cards/creatures/mv_3/akki_drillmaster.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Akki Drillmaster is a `{2}{R}` 2/2 whose whole text is "`{T}`: Target
/// creature gains haste until end of turn." Two Llanowar Elves are cast in one
/// main phase, so both are summoning sick (CR 302.6) and neither may attack —
/// and the Drillmaster, which arrived a turn earlier so that its own `{T}` is
/// payable, is aimed at exactly one of them. The attack declaration then has to
/// name one Elf and not the other, which is haste doing its printed job rather
/// than a keyword projected onto a card: the untargeted twin stands on the same
/// board, cast in the same phase, with nothing missing from it, and the Elf
/// across the table is what shows the printed "target creature" is not "target
/// creature you control".
#[test]
fn akki_drillmaster_gives_one_of_two_freshly_cast_creatures_haste_and_leaves_the_other_sick() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), forest(), forest()])
        .hand(0, &[akki_drillmaster(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {2}{R} off the three Mountains. The Drillmaster arrives sick, so its own
    // `{T}` waits for the untap step that clears CR 302.6.
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, akki_drillmaster());
    pass_until(&mut engine, stack_is_empty);
    let drillmaster =
        on_battlefield(&engine, p0, akki_drillmaster()).expect("the Drillmaster resolved");
    assert_eq!(pt(&engine, drillmaster), (2, 2), "the printed 2/2");

    // A full turn cycle, so the board stands back up and both Elves are cast in
    // *this* main phase — which is exactly what makes them summoning sick.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "three Mountains and two Forests, and the Drillmaster makes no mana"
    );
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, both of them cast this turn");
    let (sick, untouched) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, sick).contains(KeywordSet::HASTE)
            && !keywords(&engine, untouched).contains(KeywordSet::HASTE),
        "nothing on this board grants haste yet"
    );

    activate(&mut engine, p0, akki_drillmaster(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the ability asks for a creature, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert!(
        options.contains(&sick) && options.contains(&untouched),
        "\"target creature\" offers both of mine: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"target creature\" reaches across the table, so it is not \"target \
         creature you control\": {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![sick],
            },
        )
        .expect("the creature the question offered");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, sick).contains(KeywordSet::HASTE),
        "the targeted creature has haste until end of turn"
    );
    assert!(
        !keywords(&engine, untouched).contains(KeywordSet::HASTE),
        "and the Elf nobody named does not"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::HASTE),
        "nor does the Elf across the table"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&sick),
        "a creature that arrived this turn may attack only because the grant \
         lifted CR 302.6: {attackers:?}"
    );
    assert!(
        !attackers.contains(&untouched),
        "its twin, cast in the same phase and granted nothing, is still \
         summoning sick: {attackers:?}"
    );
    assert!(
        is_tapped(&engine, drillmaster),
        "and {{T}} was the price the Drillmaster actually paid"
    );
}
