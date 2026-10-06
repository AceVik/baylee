//! `cards/instants/mv_1/stand_firm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Stand Firm prints two sentences: "Target creature gets +1/+1 until end of
/// turn." and "Scry 2." Both are only worthwhile together, so next to the
/// bearer there is a second Elf on the table — "target creature" is any
/// creature, and `(2, 2)` versus `(1, 1)` rules out "creatures you control"
/// at the same time. The scry is read as *movement* and not as a posed
/// question: the chosen card is then on the bottom, the other is the new top,
/// and the library is as long as before — scry looks and sorts, it draws
/// nothing.
#[test]
fn stand_firm_pumps_the_creature_it_names_and_scries_two() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), llanowar_elves()])
        .hand(0, &[stand_firm()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");

    // The two cards the scry is about to look at, named before anything is
    // cast. The list's last entry is the top of the library — the order
    // `Effect::Scry` reads the top `n` in, and the end `ZonePosition::Bottom`
    // writes to.
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    let second = library_before[library_before.len() - 2];

    cast_from_hand(&mut engine, p0, stand_firm());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
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
        .expect("the creature the question offered was chosen");
    // CR 601.2c picks the target while the spell is still being cast, so
    // nothing has resolved yet: the Elf is still the 1/1 it was printed as.
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "the target is named before the spell resolves"
    );

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
    assert_eq!(prompt, crate::choice::ArrangePrompt::Scry);
    assert_eq!(cards, vec![top, second], "the top two cards, top first");
    assert_eq!(
        piles,
        scry_piles(2),
        "either, both or neither may be bottomed"
    );

    engine
        .apply(p0, look_answer(&cards, &[top]))
        .expect("a card the scry put on the menu is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert_eq!(
        library.first().copied(),
        Some(top),
        "the chosen card is bottomed"
    );
    assert_eq!(
        library.last().copied(),
        Some(second),
        "the other is the new top"
    );
    assert_eq!(
        library.len(),
        library_before.len(),
        "scry draws nothing, so the library is the length it was"
    );

    assert_eq!(
        pt(&engine, mine),
        (2, 2),
        "+1/+1 until end of turn on the creature the spell named"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and nothing at all for the Elf it did not"
    );
}
