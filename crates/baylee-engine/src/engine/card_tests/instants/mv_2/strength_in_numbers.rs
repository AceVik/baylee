//! `cards/instants/mv_2/strength_in_numbers.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Strength in Numbers is `{1}{G}` for "Until end of turn, target creature
/// gains trample and gets +X/+X, where X is the number of attacking
/// creatures." X is the whole card, so the spell is aimed at the one Elf that
/// stayed home while two others attack: a home 1/1 reading 3/3 can only have
/// counted the attackers, where aiming at an attacker itself could not tell
/// X = 1 from X = 2 or even from 0. The two attackers and the Elf across the
/// table are the controls for "target creature" — neither may change — and
/// walking a whole turn afterwards is the control for the printed "until end of
/// turn", which no single-moment reading can see.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn strength_in_numbers_counts_the_attackers_for_a_creature_that_is_not_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        // A creature across the table, so "target creature" has a second side
        // to reach and a bystander that must stay a printed 1/1.
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[strength_in_numbers()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 3, "three Elves: two attack, one stays home");
    let (first, second) = (elves[0], elves[1]);
    let home = elves[2];
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, home), (1, 1), "a printed 1/1 before the spell");
    assert!(
        !keywords(&engine, home).contains(KeywordSet::TRAMPLE),
        "and it starts with no trample for the spell to be given credit for"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert_eq!(player, p0, "the active seat declares its attackers");
    assert!(
        attackers.contains(&first) && attackers.contains(&second),
        "the two untapped Elves are on the offer: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (first, Defender::Player(p1)),
                    (second, Defender::Player(p1)),
                ],
            },
        )
        .expect("both attackers came out of the list that offered them");

    // The instant is played in the step it counts, and the mana is made there
    // too: a pool empties when a step ends (CR 500.5), so the Forests cannot be
    // tapped in the main phase and carried into the attack. Both attackers are
    // tapped by now and the third Elf is named as the printing kept back — it is
    // the creature the spell is about to be aimed at.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests and no Elf of mine: {{G}}{{G}} is the whole pool"
    );

    let spell = in_hand(&engine, p0, strength_in_numbers()).expect("the instant is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&spell),
        "{{1}}{{G}} is in the pool, so the instant is playable in the middle of \
         the combat step: {:?}",
        legal.castable
    );
    cast_with_floating(&mut engine, p0, strength_in_numbers());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one target, and the spell asks once");
    assert!(
        options.contains(&home) && options.contains(&first) && options.contains(&second),
        "\"target creature\" is any creature, the two attackers included: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "and it reaches across the table, which a filter carrying \
         `ControlledByYou` would not do: {options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so the
    // mana is still floating while this question stands.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the cost is the last step of the cast, not the first"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![home],
            },
        )
        .expect("the creature the question offered was chosen");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and then the {{1}}{{G}} came out of the pool"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, home),
        (3, 3),
        "+X/+X with X the number of attacking creatures, which is two — on a \
         creature that is not one of them, so a 1 or a 0 would have to have come \
         from somewhere other than the printed count"
    );
    assert!(
        keywords(&engine, home).contains(KeywordSet::TRAMPLE),
        "and the printed trample reaches the same creature"
    );
    assert_eq!(
        pt(&engine, first),
        (1, 1),
        "the attackers the count was made of are untouched: the effect targets, \
         it does not sweep the board"
    );
    assert_eq!(pt(&engine, second), (1, 1), "and neither is the other one");
    assert!(
        !keywords(&engine, first).contains(KeywordSet::TRAMPLE)
            && !keywords(&engine, second).contains(KeywordSet::TRAMPLE),
        "nor was either of them granted a keyword it never asked for"
    );
    assert_eq!(pt(&engine, theirs), (1, 1), "and nothing crosses the table");
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::TRAMPLE),
        "the Elf on the other side is a target the spell could have named and \
         did not"
    );

    // "until end of turn": a whole turn later the Elf is a printed 1/1 with no
    // trample, which is the only reading that tells the printed duration from a
    // permanent grant.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, home),
        (1, 1),
        "the pump lasted the turn it was cast in and no longer"
    );
    assert!(
        !keywords(&engine, home).contains(KeywordSet::TRAMPLE),
        "and the trample left with it"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the creature is still standing, so the grant left rather than the creature"
    );
    assert!(
        in_graveyard(&engine, p0, strength_in_numbers()).is_some(),
        "an instant is in its owner's graveyard once it has resolved"
    );
}
