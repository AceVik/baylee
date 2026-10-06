//! `cards/sorceries/mv_3/eldritch_evolution.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Eldritch Evolution — {1}{G}{G} sorcery: "As an additional cost to cast
/// this spell, sacrifice a creature. Search your library for a creature card
/// with mana value X or less, where X is 2 plus the sacrificed creature's
/// mana value. Put that card onto the battlefield, then shuffle. Exile
/// Eldritch Evolution."
///
/// The library is Land Leeches, mana value 3, and the spell is cast twice.
/// Sacrificing Ornithopter (mana value 0) makes X 2, and the search finds
/// nothing; sacrificing the Elves (1) makes X 3, and it finds Leeches. The
/// pair is what shows the bound is read off the creature that was paid, and
/// that it is "or less" and "2 plus": a bound that ignored the sacrifice, or
/// added 1, gives a different pair of answers.
#[test]
fn eldritch_evolution_bounds_its_search_by_the_sacrificed_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, land_leeches())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                ornithopter(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[eldritch_evolution(), eldritch_evolution()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let thopter = on_battlefield(&engine, p0, ornithopter()).unwrap();
    let library_before = library_size(&engine, p0);

    cast_from_hand(&mut engine, p0, eldritch_evolution());
    sacrifice_as_cast(&mut engine, p0, thopter);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        fielded(&engine, p0, land_leeches()),
        0,
        "X = 2 + 0 finds no mana value 3 creature"
    );
    assert_eq!(library_size(&engine, p0), library_before);
    assert_eq!(exiled(&engine, p0, eldritch_evolution()), 1);
    assert!(in_graveyard(&engine, p0, eldritch_evolution()).is_none());

    cast_with_floating(&mut engine, p0, eldritch_evolution());
    sacrifice_as_cast(&mut engine, p0, elves);
    let Pending::ChooseCards { options, min, .. } = pass_to_card_choice(&mut engine) else {
        unreachable!("the helper returns only a card choice")
    };
    assert_eq!(min, 1);
    assert_eq!(
        options.len(),
        library_before,
        "X = 2 + 1 offers every Leeches"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(fielded(&engine, p0, land_leeches()), 1);
    assert_eq!(exiled(&engine, p0, eldritch_evolution()), 2);
}
