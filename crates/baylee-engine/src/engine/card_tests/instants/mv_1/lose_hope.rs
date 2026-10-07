//! `cards/instants/mv_1/lose_hope.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lose Hope is a black instant with two sentences — "Target
/// creature gets -1/-1 until end of turn" and "Scry 2" — and both are
/// played. The target is the Wurm on the table, and *both* of its
/// numbers are read: the difference `(power - 1, toughness - 1)` is the
/// only pair that evidences a -1/-1, while a -0/-1 would leave the power
/// unchanged; that the Wurm survives is also the reason why the numbers
/// are readable at all — a 1/1 would have died according to CR 704.5f
/// before anyone could have checked its power. The Elf next to it is the
/// other half of "Target creature": exactly one target is available, and
/// the creature that the spell did not name does not move. The
/// Scry half is read as a question and not as an answer — the two
/// topmost cards are up for choice, topmost first, and sending none of
/// them to the bottom leaves the library exactly as it was.
#[test]
fn lose_hope_shrinks_the_creature_it_names_and_then_scries_two() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp()])
        .hand(0, &[lose_hope()])
        .battlefield(1, &[rootbreaker_wurm(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("their Wurm is out");
    let bystander = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf beside it");
    let (power, toughness) = pt(&engine, wurm);
    let untouched = pt(&engine, bystander);

    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    let second = library_before[library_before.len() - 2];

    cast_from_hand(&mut engine, p0, lose_hope());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster aims it");
    assert!(
        options.contains(&wurm) && options.contains(&bystander),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and the two creatures over there are the whole menu: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .expect("the Wurm was one of the options");

    // Die Auflösung führt durch den ersten Satz hindurch zur Frage des
    // zweiten: erst schrumpfen, dann schauen.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the caster does the looking");
    assert_eq!(
        prompt,
        ArrangePrompt::Scry,
        "scry is its own question and not a search or a discard"
    );
    assert_eq!(
        piles,
        scry_piles(2),
        "Scry 2: either, both or neither of the top two, and none of them forced"
    );
    assert_eq!(cards, vec![top, second], "the top two cards, top first");

    engine
        .apply(p0, look_answer(&cards, &[]))
        .expect("looking is not moving: keeping both on top is a legal answer");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        pt(&engine, wurm),
        (power - 1, toughness - 1),
        "-1/-1 on the creature the spell named — a (power, toughness - 1) \
         would mean only the toughness was ever read"
    );
    assert!(
        on_battlefield(&engine, p1, rootbreaker_wurm()).is_some(),
        "and it survives: the spell shrinks a creature rather than destroying one"
    );
    assert_eq!(
        pt(&engine, bystander),
        untouched,
        "the creature the spell did not name never moved"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).clone(),
        library_before,
        "scry 2 looks and reorders: nothing was drawn and nothing was bottomed"
    );
}
