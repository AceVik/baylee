//! `cards/instants/mv_2/guided_strike.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Guided Strike — {1}{W} instant: "Target creature gets +1/+0 and gains first
/// strike until end of turn. Draw a card."
///
/// Both sentences are one resolution, so the card has to be cast to be read at
/// all. "Target creature" names no controller, so the offer is taken with an
/// Elf standing on *each* side of the table: a 1/1 becoming `(2, 1)` is what
/// says the `+1/+0` was applied once and the toughness left alone, while the
/// Elf across the table staying a printed, keywordless 1/1 is what says the
/// pump reached the creature that was named and not the board. The draw is the
/// half no characteristic can show, so it is read as the library getting
/// shorter and the instant landing in its owner's graveyard.
#[test]
fn guided_strike_pumps_one_creature_and_draws_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[guided_strike()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the spell");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::FIRST_STRIKE),
        "and it has no first strike of its own"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Two Plains pay {1}{W}. The Elf is kept back because it is the creature
    // the spell is about to name, and a source tapped for its own mana has
    // already changed for a reason this test is not reading.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Plains in the pool, and the Elf paid nothing"
    );
    cast_with_floating(&mut engine, p0, guided_strike());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("the pump targets a creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the caster aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("my own Elf was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (2, 1),
        "+1/+0 on the creature the spell named: power up, toughness untouched"
    );
    assert!(
        keywords(&engine, mine).contains(KeywordSet::FIRST_STRIKE),
        "and the printed first strike reaches it through the layers"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the creature it targeted and no other"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FIRST_STRIKE),
        "nor does the keyword cross the table"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the spell left the hand and the draw replaced it, so the hand is the \
         size it was"
    );
    assert!(
        in_graveyard(&engine, p0, guided_strike()).is_some(),
        "and the instant itself is in its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{W}} came out of the pool"
    );
}
