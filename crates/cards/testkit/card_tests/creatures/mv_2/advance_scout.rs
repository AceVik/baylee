//! `cards/creatures/mv_2/advance_scout.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Advance Scout is a 1/1 with printed first strike and `{W}: Target
/// creature gains first strike until end of turn`. The scenario is built
/// around the two words a wrong implementation gets wrong: "target
/// creature" reaches either side of the table, so the opponent's creature
/// stands on the menu and has to be left keywordless when the other one is
/// named; and the effect is a 0/0 pump that hands over a keyword alone, so
/// the target's body reads exactly what it read before and the loan is over
/// by the next main phase.
#[test]
fn advance_scout_lends_its_first_strike_to_the_creature_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), advance_scout(), quiet_creature()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let scout = on_battlefield(&engine, p0, advance_scout()).expect("the Scout is out");
    let mine = on_battlefield(&engine, p0, quiet_creature()).expect("my creature is out");
    let theirs = on_battlefield(&engine, p1, quiet_creature()).expect("their creature is out");

    assert!(
        keywords(&engine, scout).contains(KeywordSet::FIRST_STRIKE),
        "the keyword is printed on the Scout itself"
    );
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::FIRST_STRIKE),
        "and on neither creature beside it"
    );
    assert!(!keywords(&engine, theirs).contains(KeywordSet::FIRST_STRIKE));

    // The Elf is kept back: it is the creature the ability is about to name,
    // and a target tapped for mana would be a target that had already spent
    // its turn. Two Plains, two white.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "only the two Plains paid in"
    );

    // Ability 0 is the only activated ability the card prints.
    activate(&mut engine, p0, advance_scout(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
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
        .expect("the creature the question offered");
    // CR 601.2h: paying is the last step of casting, so the {W} leaves the
    // pool once the target is named and not before.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{W}} left the pool"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, mine).contains(KeywordSet::FIRST_STRIKE),
        "the creature it named gains first strike"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "a 0/0 pump: the keyword moved and the body did not"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FIRST_STRIKE),
        "the ability reaches the creature it named and never across the table"
    );
    assert!(
        keywords(&engine, scout).contains(KeywordSet::FIRST_STRIKE),
        "and the Scout keeps its own"
    );

    // "until end of turn": a whole turn cycle later the loan is over.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::FIRST_STRIKE),
        "the keyword was lent for the turn and not granted for good"
    );
}
