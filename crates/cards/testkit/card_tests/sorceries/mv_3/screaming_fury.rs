//! `cards/sorceries/mv_3/screaming_fury.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Screaming Fury — {2}{R} sorcery: "Target creature gets +5/+0 and gains
/// haste until end of turn." None of that sentence is visible in the card
/// file, so one board plays all of it at once: three Mountains are the mana,
/// the Elf that is named is a printed 1/1 that becomes a 6/1 carrying the
/// keyword, and the Elf across the table beside a Sol Ring is what tells
/// "target creature" from "target creature you control" and from "target
/// permanent". A whole turn is walked afterwards, because "until end of turn"
/// is half the card and a grant that never expired would pass every
/// assertion above it.
#[test]
#[allow(clippy::too_many_lines)]
fn screaming_fury_pumps_the_creature_it_names_and_grants_it_haste_until_the_turn_ends() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), llanowar_elves()])
        // A creature and an artifact this seat does not control, so the menu
        // has both halves of the filter to answer for.
        .battlefield(1, &[llanowar_elves(), quiet_artifact()])
        .hand(0, &[screaming_fury()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "a sorcery needs p0's own main phase with an empty stack"
    );

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the spell");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::HASTE),
        "nothing has granted anything yet"
    );

    // Three Mountains are exactly {2}{R}. The Elf is named as the printing
    // kept back, so the pool below is the three red and nothing else — its
    // own {T}: Add {G} would otherwise sit in the same count.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three tapped Mountains and an untapped Elf"
    );
    cast_with_floating(&mut engine, p0, screaming_fury());

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
    assert_eq!(player, p0, "the seat that cast the spell names its target");
    assert_eq!((min, max), (1, 1), "one creature, and the spell asks once");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&rock),
        "a Sol Ring is an artifact and no creature: {options:?}"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "the pump is the resolution and not the announcement: while the \
         question stands the creature is still the 1/1 it was printed as"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the Elf was one of the options it enumerated");
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so the spell is waiting to resolve"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (6, 1),
        "+5/+0 on the creature the spell named"
    );
    assert!(
        keywords(&engine, mine).contains(KeywordSet::HASTE),
        "and the printed haste, read through the layers"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the creature it targeted and never across the table"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::HASTE),
        "nor does the keyword sweep the board"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}}{{R}} came out of the pool"
    );

    // "until end of turn": a turn later the Elf is a printed 1/1 again, so
    // the pump was a duration and not a body the board keeps.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "the grant lasted the turn it was cast in and no longer"
    );
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::HASTE),
        "and the haste left with it"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the creature is still standing, so the pump left rather than the creature"
    );
}
