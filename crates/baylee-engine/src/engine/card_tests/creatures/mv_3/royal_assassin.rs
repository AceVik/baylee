//! `cards/creatures/mv_3/royal_assassin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Royal Assassin — {1}{B}{B}, a 1/1 Human Assassin whose entire text is
/// "{T}: Destroy target tapped creature."
///
/// Two words carry the card and each needs its own witness, so four creatures
/// stand: a tapped one and an untapped one on each side of the table, none of
/// them bought with a combat step. Only the two tapped creatures may appear in
/// the offer — a list carrying an untapped one would mean "tapped" was never
/// read, and a list holding nothing but this seat's own creature would mean a
/// "you control" the card does not print. Killing the Elf across the table
/// with a price that is nothing but the tap symbol is then the whole card: the
/// named creature leaves for its owner's graveyard while the untapped Elf
/// beside it, the tapped Elf behind it and the Assassin itself all stay put.
#[test]
#[allow(clippy::too_many_lines)] // two taps, one activation, every clause of the card read off it
fn royal_assassin_destroys_the_tapped_creature_it_names_and_reaches_across_the_table() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[royal_assassin(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let assassin = on_battlefield(&engine, p0, royal_assassin()).expect("the Assassin is out");
    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(theirs.len(), 2, "two Elves across the table");
    let (standing_elf, doomed) = (theirs[0], theirs[1]);

    // Two creatures tapped and two not, so the offer below has something to be
    // wrong about in either direction. Tapping them for their own mana is the
    // shortest way to say "tapped" without a combat step, and `tap_mana_where`
    // will not return while a printed mana ability is still standing untapped
    // (#159) — a creature's is one of them.
    tap_mana_where(&mut engine, p0, |id| id == my_elf);
    assert!(is_tapped(&engine, my_elf), "my own Elf is tapped for mana");
    // The Elf across the table has to be tapped by its own controller, so
    // priority is handed over and then straight back: a mana ability is an
    // action, and the round starts over after one.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_mana_where(&mut engine, p1, |id| id == doomed);
    assert!(
        is_tapped(&engine, doomed),
        "and one across the table is too"
    );
    assert!(
        !is_tapped(&engine, standing_elf),
        "the other Elf never moved"
    );
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet priority: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(assassin, 0)),
        "an untapped Assassin is a paid {{T}}, so the one line the card prints \
         is offered: {:?}",
        legal.abilities
    );
    let pool_before = engine.state().players[0].mana_pool.total();

    activate(&mut engine, p0, royal_assassin(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target tapped creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert_eq!(
        options.len(),
        2,
        "exactly the two tapped creatures on the battlefield: {options:?}"
    );
    assert!(
        options.contains(&doomed),
        "\"tapped creature\" is not \"a creature you control\": the Elf across \
         the table is on the menu: {options:?}"
    );
    assert!(
        options.contains(&my_elf),
        "and so is the tapped creature this seat controls: {options:?}"
    );
    assert!(
        !options.contains(&standing_elf),
        "the Elf that is still untapped is the whole of what \"tapped\" \
         excludes: {options:?}"
    );
    assert!(
        !options.contains(&assassin),
        "an untapped creature may not be aimed at, its own ability \
         notwithstanding: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![doomed],
            },
        )
        .expect("the Elf the question offered was one of its answers");

    assert!(
        is_tapped(&engine, assassin),
        "{{T}} is the whole price and is paid as the ability is activated"
    );
    assert!(
        !stack_is_empty(&engine),
        "destroying a creature is no mana ability, so it is on the stack"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        pool_before,
        "and nothing about the price is mana"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the targeted creature died, and it is in its *owner's* graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the Elf the ability did not name is still across the table"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and so is the tapped creature this seat controls: one target, one \
         casualty"
    );
    assert!(
        on_battlefield(&engine, p0, royal_assassin()).is_some(),
        "the Assassin outlives what it killed"
    );
}
