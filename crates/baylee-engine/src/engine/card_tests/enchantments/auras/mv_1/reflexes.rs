//! `cards/enchantments/auras/mv_1/reflexes.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Reflexes prints two sentences: "Enchant creature" and "Enchanted creature
/// has first strike." This scenario plays both. The Aura is cast off a
/// Mountain at the only two creatures on the table, so "enchant creature"
/// is read as *any* creature rather than "you control" — the Elf across the
/// table is offered and the one under our own seat is taken. The keyword is
/// then proven in combat instead of in a keyword list: two printed 1/1s
/// would trade, and only the one striking first leaves the blocking 1/1 dead
/// while the attacker lives.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn reflexes_enchants_a_creature_and_lets_it_strike_first() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, mountain())
        .battlefield(0, &[mountain(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[reflexes()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::FIRST_STRIKE),
        "nothing is enchanted yet"
    );

    // The Mountain pays the {R}; the Elf is kept back because it is the
    // creature that has to attack in the combat step below.
    tap_mana_except(&mut engine, p0, mine);
    cast_with_floating(&mut engine, p0, reflexes());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "an Aura targets a creature as it is cast, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat chooses the target");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"enchant creature\" is any creature, on either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    let aura = on_battlefield(&engine, p0, reflexes()).expect("the Aura resolved onto the table");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(mine),
        "it entered attached to the creature it was aimed at"
    );
    assert!(
        keywords(&engine, mine).contains(KeywordSet::FIRST_STRIKE),
        "the enchanted creature has first strike"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FIRST_STRIKE),
        "and the creature it is not attached to has none"
    );

    // Combat is where first strike means anything: the two 1/1s would trade,
    // and only the one that strikes first survives.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(mine, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p1, "the defending seat declares the blocks");
    assert!(
        blockers
            .iter()
            .any(|option| option.blocker == theirs && option.attackers.contains(&mine)),
        "the Elves across the table may block the attacker: {blockers:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(theirs, mine)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the first striker deals its damage first, so the blocker dies before \
         it can deal any back and the 1/1 attacker lives"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "and the blocking 1/1 is in its owner's graveyard"
    );
}
