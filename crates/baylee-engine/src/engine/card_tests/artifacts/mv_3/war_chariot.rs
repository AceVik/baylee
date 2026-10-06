//! `cards/artifacts/mv_3/war_chariot.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// War Chariot prints one line — "{3}, {T}: Target creature gains trample
/// until end of turn" — and both halves of that price leave a mark a test can
/// read: the {3} leaves the pool and the {T} leaves the artifact tapped.
/// "Target creature" is any creature on either side of the table, so an Elf
/// across it is offered the keyword and must finish without it, while a second
/// Elf of mine stands beside the one that was named and must stay bare too —
/// the pair is what tells a target from a board-wide grant. The turn is walked
/// to an end because the printed duration is part of the card: a keyword that
/// never expires would pass every assertion above it.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn war_chariot_grants_trample_to_the_creature_it_names_and_only_until_the_turn_ends() {
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
                forest(),
                llanowar_elves(),
                llanowar_elves(),
                war_chariot(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let chariot = on_battlefield(&engine, p0, war_chariot()).expect("the Chariot is out");
    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays the control");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::TRAMPLE),
        "nothing has granted anything yet"
    );

    // `legal.abilities` is filtered through `can_afford`, and that reads the
    // *pool* rather than the untapped lands: with nothing floating the {3} is
    // unpayable and the line is not there at all — the half a test that only
    // ever taps first would never see.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(chariot, 0)),
        "{{3}} is not three, so the cost is unpayable and nothing is offered: {:?}",
        legal.abilities
    );

    // Six Forests, and both Elves named as the printing kept back: they are
    // the creatures the ability is about to choose between, and a mana
    // creature tapped for the cost would make "six" a count of seven.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six tapped Forests and two untapped Elves"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(chariot, 0)),
        "with six floating the whole price is payable: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, war_chariot(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert!(
        options.contains(&host) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&chariot),
        "the Chariot is an artifact and no creature: {options:?}"
    );

    // CR 601.2c before CR 601.2h: while the question stands, the mana is
    // still floating and the artifact is still untapped.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "the cost is the last step of the activation, so nothing is spent yet"
    );
    assert!(
        !is_tapped(&engine, chariot),
        "and nothing has tapped it yet"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Elf was one of the options the question enumerated");

    assert!(
        is_tapped(&engine, chariot),
        "{{T}} is the other half of the price"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "and the {{3}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "granting a keyword is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, host).contains(KeywordSet::TRAMPLE),
        "the creature the ability named gained trample"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::TRAMPLE),
        "the Elf nobody named is untouched: the effect targets, it does not sweep the board"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::TRAMPLE),
        "and it never reaches across the table"
    );
    assert!(
        !keywords(&engine, chariot).contains(KeywordSet::TRAMPLE),
        "the Chariot grants the keyword, it does not keep it"
    );

    // "until end of turn": the Elf is still there a turn later and the keyword
    // is not — a static or a permanent grant would still be on it here.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !keywords(&engine, host).contains(KeywordSet::TRAMPLE),
        "the grant lasted the turn it was made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the creature is still standing, so the keyword left rather than the creature"
    );
}
