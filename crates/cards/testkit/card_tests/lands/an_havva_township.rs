//! `cards/lands/an_havva_township.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// An-Havva Township prints three mana abilities and the whole card is the
/// difference between them: `{T}` for `{C}`, `{1}` **and** a tap for `{G}`,
/// `{2}` and a tap for `{R}` or `{W}`. Reading the file says all three exist;
/// only a board says which of them the engine offers and what each puts in the
/// pool — so the land is played (it prints no enters-tapped clause, which is
/// what makes its own tap usable the turn it arrives), two more stand beside
/// it, and two Sol Rings are the only mana the two paid lines may be paid
/// with. No green is on that board at all, so the green after the `{1}` line
/// can have come from nowhere else, and the `{2}` line's question is read
/// before it is answered.
#[test]
#[allow(clippy::too_many_lines)] // three printed lines, each read off one activation
fn an_havva_township_taps_for_colorless_then_buys_green_and_a_choice_of_red_or_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, basic_forest())
        .battlefield(
            0,
            &[
                an_havva_township(),
                an_havva_township(),
                quiet_artifact(),
                quiet_artifact(),
            ],
        )
        .hand(0, &[an_havva_township()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The land drop, and with it the reading that nothing about the card makes
    // it arrive tapped: the permanent is standing and payable in the same
    // phase it was played in.
    let played = play_land(&mut engine, p0, an_havva_township());
    let townships = all_on_battlefield(&engine, p0, an_havva_township());
    assert_eq!(townships.len(), 3, "three copies, one of them just played");
    assert!(townships.contains(&played), "and it is among them");
    assert!(
        !is_tapped(&engine, played),
        "a land with no enters-tapped clause is usable the turn it arrives"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "an empty pool, so the offer below is read off the costs and nothing else"
    );

    // The control for the two paid lines: `legal.abilities` is filtered through
    // `can_afford`, which reads the pool and not the untapped lands, so this
    // board offers the free line alone — and the same board with four floating
    // offers all three.
    let offers = |engine: &Engine<RegistryLookup>, index: u32| {
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        townships
            .iter()
            .filter(|land| legal.abilities.contains(&(**land, index)))
            .count()
    };
    assert_eq!(offers(&engine, 0), 3, "`{{T}}: Add {{C}}` costs no mana");
    assert_eq!(offers(&engine, 1), 0, "and the `{{1}}` line cannot be paid");
    assert_eq!(offers(&engine, 2), 0, "nor the `{{2}}` line");

    // Two Sol Rings, taken by the helper because their whole price is their own
    // tap. The Township is named so that all three copies stay standing: each
    // is a permanent whose other lines are what this test is about, and one
    // tapped for mana could not pay a tap symbol afterwards.
    tap_all_mana_but(&mut engine, p0, Some(an_havva_township()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        4,
        "two Sol Rings, four colourless — and no green anywhere on this board"
    );
    for index in [0u32, 1, 2] {
        assert_eq!(
            offers(&engine, index),
            3,
            "four colourless afford all three printed lines at once"
        );
    }

    // `{T}: Add {C}`.
    activate(&mut engine, p0, an_havva_township(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        5,
        "four off the Sol Rings and one off the land's own tap"
    );
    assert_eq!(pool.available(ManaColor::Green), 0, "and nothing green yet");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );

    // `{1}, {T}: Add {G}` — the line whose green has no other possible origin.
    activate(&mut engine, p0, an_havva_township(), 1);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "one green, and there was no green on this board for it to be confused with"
    );
    assert_eq!(
        pool.available(ManaColor::Colorless),
        4,
        "the {{1}} came out of the pool: five in, one spent, one made"
    );

    // `{2}, {T}: Add {R} or {W}` — `mana_choice` asks, and what it asks about
    // is red and white; the green the line above prints is not one of them.
    activate(&mut engine, p0, an_havva_township(), 2);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{R}} or {{W}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that activated names the colour");
    assert_eq!(
        options.len(),
        2,
        "the two colours the line prints, and no colourless: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Red) && options.contains(&ManaColor::White),
        "red or white, exactly: {options:?}"
    );
    assert!(
        !options.contains(&ManaColor::Green),
        "the green belongs to the {{1}} line, and this is not that line: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white was one of the two colours the ability offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the colour that was named, and not the other one"
    );
    assert_eq!(pool.available(ManaColor::Red), 0, "red was not named");
    assert_eq!(
        pool.total(),
        4,
        "five colourless in, the {{1}} and the {{2}} spent, one green and one white made"
    );
    assert!(
        stack_is_empty(&engine),
        "still no stack: all three lines are mana abilities (CR 605.3b)"
    );
    assert!(
        townships.iter().all(|land| is_tapped(&engine, *land)),
        "and each activation was paid with the tap symbol of a different copy"
    );
}
