//! `cards/sorceries/mv_3/entreat_the_dead.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Entreat the Dead — {X}{X}{B}{B}{B} — "return X target creature cards from
/// your graveyard to the battlefield."
///
/// X is the count of *targets* rather than a number the effect reads, which
/// is the one shape a reanimation written against the first target cannot
/// do at all: the announced two would have brought one card back and the
/// spell would have looked like it worked. So the board holds exactly two
/// creature cards and the assertion is both of them.
///
/// Seven Swamps, because {X}{X} at X = 2 is four mana in front of {B}{B}{B}
/// — the doubled symbol is what makes a wrong X visible as a refused cast
/// rather than as a cheaper one.
#[test]
fn entreat_the_dead_returns_as_many_creatures_as_the_x_it_was_cast_for() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(44, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                llanowar_elves(),
                rib_cage_spider(),
            ],
        )
        .hand(0, &[entreat_the_dead()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are seated");
    let spider = on_battlefield(&engine, p0, rib_cage_spider()).expect("the Spider is seated");
    bury(&mut engine, &[elves, spider]);

    cast_from_hand(&mut engine, p0, entreat_the_dead());
    let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() else {
        panic!(
            "a printed {{X}} has to be asked about: {:?}",
            engine.pending()
        )
    };
    assert_eq!(min, 0, "X may always be nothing");
    assert!(
        max >= 2,
        "seven Swamps pay {{X}}{{X}}{{B}}{{B}}{{B}} for X = 2: max = {max}"
    );
    engine
        .apply(p0, PlayerAction::ChooseNumber(2))
        .expect("X = 2 is inside the range the engine just offered");

    let options = pass_until_targets(&mut engine, p0);
    assert!(
        options.contains(&elves) && options.contains(&spider),
        "both creature cards in my graveyard: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elves, spider],
                players: vec![],
            },
        )
        .expect("X targets, and X was announced as two");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the first of the two came back"
    );
    assert!(
        on_battlefield(&engine, p0, rib_cage_spider()).is_some(),
        "and so did the second — X is a count and not a decoration"
    );
    assert!(
        in_graveyard(&engine, p0, entreat_the_dead()).is_some(),
        "and the sorcery itself resolved into the graveyard"
    );
}
