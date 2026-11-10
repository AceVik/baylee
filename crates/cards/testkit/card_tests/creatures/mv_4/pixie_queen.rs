//! `cards/creatures/mv_4/pixie_queen.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pixie Queen — {2}{G}{G} — Creature — Faerie: Flying, and "{G}{G}{G}, {T}:
/// Target creature gains flying until end of turn."
///
/// The printed keyword is hers and the grant is not, so the board carries a
/// second creature of mine and one across the table: "target creature" is one
/// creature, and the Elf nobody names has to stay grounded while the Elf that
/// was named takes the keyword. Both halves of the price are read where they
/// land — the three Forests leave an exact pool and the {T} leaves the Queen
/// tapped — and the printed "until end of turn" gets a turn of its own, because
/// a grant that never expires would pass every assertion above it. She is
/// seated before the game starts rather than cast: a creature that arrived
/// this turn may not pay a {T} (CR 302.6), and the tap is half of what this
/// test reads.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn pixie_queen_spends_three_green_and_her_own_tap_for_flying_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                pixie_queen(),
                forest(),
                forest(),
                forest(),
                quiet_creature(),
                quiet_creature(),
            ],
        )
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let queen = on_battlefield(&engine, p0, pixie_queen()).expect("the Queen is seated");
    let elves = all_on_battlefield(&engine, p0, quiet_creature());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays the control");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, quiet_creature()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::FLYING),
        "nothing has granted anything yet"
    );
    assert!(
        keywords(&engine, queen).contains(KeywordSet::FLYING),
        "Flying is the keyword the card prints"
    );
    assert!(
        !is_tapped(&engine, queen),
        "and she stands untapped, so the {{T}} her ability charges is still there to pay"
    );

    // `legal.abilities` is filtered through `can_afford`, which reads the pool
    // and not the untapped lands, so before a single Forest is tapped the line
    // is not on offer at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(queen, 0)),
        "an empty pool pays no {{G}}{{G}}{{G}}: {:?}",
        legal.abilities
    );

    // Three Forests, with both Elves named as the printing kept back: the
    // ability's price is {G}{G}{G} and no creature, so a bystander already
    // tapped for mana would be a different board.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three tapped Forests and no Elf: exactly the {{G}}{{G}}{{G}} the ability charges"
    );

    // The same offer read again, now that the mana is floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(queen, 0)),
        "with {{G}}{{G}}{{G}} in the pool the one line the card prints is \
         offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, pixie_queen(), 0);
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
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one target, and the ability asks once");
    assert!(
        options.contains(&host) && options.contains(&bystander) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    // CR 601.2c before CR 601.2h: while the question stands, the three green
    // are still floating and the Queen is still standing.
    assert!(
        !is_tapped(&engine, queen),
        "the {{T}} is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "and the {{G}}{{G}}{{G}} is still in the pool for the same reason"
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
        is_tapped(&engine, queen),
        "{{T}} is the other half of the price"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{G}}{{G}}{{G}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "granting a keyword is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        keywords(&engine, host).contains(KeywordSet::FLYING),
        "the creature the ability named gained flying"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::FLYING),
        "the Elf nobody named is untouched: the effect targets, it does not \
         sweep the board"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "and it never reaches across the table for a creature that was not named"
    );

    // "until end of turn": the untap step says a turn really passed, so the
    // missing keyword below is a duration and not a game that never advanced.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, queen),
        "the Queen stands back up in her controller's untap step, so a turn \
         has passed"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_some(),
        "and the Elf is still standing, so the keyword left rather than the creature"
    );
    assert!(
        !keywords(&engine, host).contains(KeywordSet::FLYING),
        "the grant lasted the turn it was made in and no longer"
    );
}
