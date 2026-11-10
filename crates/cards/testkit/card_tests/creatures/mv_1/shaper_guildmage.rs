//! `cards/creatures/mv_1/shaper_guildmage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shaper Guildmage prints two lines on one 1/1 body — `{W}, {T}: Target
/// creature gains first strike until end of turn` and `{B}, {T}: Target
/// creature gets +1/+0 until end of turn` — and each costs its own tap, so a
/// single copy can only ever show one of them in a turn. A second copy stands
/// beside it and one scenario therefore reads both: the keyword the first
/// line hands out and the body the second one does.
///
/// "Target creature" is the phrase the board is built around: an Elf under
/// each seat is a legal target for both lines, which is what separates the
/// printed sentence from a `YOUR_CREATURE` filter — and the black line is
/// aimed across the table, where a wrong filter would still look right on my
/// own creature. The white and the black are floated first, because a printed
/// `{W}, {T}` is no mana ability and the offer is read off the pool rather
/// than off the untapped lands.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn shaper_guildmage_turns_its_two_taps_into_first_strike_and_a_point_of_power() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(607, island())
        .battlefield(
            0,
            &[
                plains(),
                swamp(),
                shaper_guildmage(),
                shaper_guildmage(),
                quiet_creature(),
            ],
        )
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mages = all_on_battlefield(&engine, p0, shaper_guildmage());
    assert_eq!(mages.len(), 2, "two copies, and neither has tapped yet");
    let mine = on_battlefield(&engine, p0, quiet_creature()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf is out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before anything");
    assert_eq!(pt(&engine, theirs), (1, 1), "and so is the one across it");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::FIRST_STRIKE),
        "nothing has handed out a keyword yet"
    );

    // The two lands, and only the lands: a `{W}, {T}` is not a whole price of
    // its own tap, so the helper leaves both Mages standing for their own.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        mages
            .iter()
            .all(|mage| legal.abilities.contains(&(*mage, 0))
                && legal.abilities.contains(&(*mage, 1))),
        "white and black are both floating, so both lines are offered on both \
         Mages: {:?}",
        legal.abilities
    );

    // {W}, {T}: first strike, on my own Elf. The offer is read before the
    // answer, because CR 601.2c picks the target and CR 601.2h pays after.
    activate(&mut engine, p0, shaper_guildmage(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the first strike line is aimed at a target, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat does the choosing");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: \
         {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, mine).contains(KeywordSet::FIRST_STRIKE),
        "the creature the white mana was aimed at has first strike"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FIRST_STRIKE),
        "and the one it was not aimed at does not"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "a keyword is no body: the first strike line pumps nothing"
    );

    // {B}, {T}: +1/+0, and this time across the table — the half a
    // `YOUR_CREATURE` filter would lose without looking wrong on my own Elf.
    activate(&mut engine, p0, shaper_guildmage(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the pump is aimed at a target, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "the black line reads the same two words the white one does: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, theirs),
        (2, 1),
        "+1/+0 on the creature the black mana was aimed at"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "and nothing at all for the creature it was not"
    );
    assert!(
        mages.iter().all(|mage| is_tapped(&engine, *mage)),
        "each line is paid for with the Mage's own tap, so both stand tapped \
         once both have been pressed"
    );
}
