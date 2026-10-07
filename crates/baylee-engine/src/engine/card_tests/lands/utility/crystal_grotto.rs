//! `cards/lands/utility/crystal_grotto.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Crystal Grotto prints three lines and one game plays all of them: "When
/// this land enters, scry 1", "{T}: Add {C}", and "{1}, {T}: Add one mana of
/// any color." The scry is read as a **move** — the card the question names
/// lies on the bottom afterwards and the library is the length it was — and
/// the two mana lines are told apart by their price, which is the only thing
/// that differs between them: on an empty pool only the colourless half is
/// offered, and the coloured one appears exactly when a mana is floating for
/// its `{1}` and then stops to ask a question five colours wide (CR 105.4).
/// The land is played through a real `PlayLand` and arrives untapped, because
/// an entry trigger is only an entry trigger if the permanent really entered.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn crystal_grotto_scries_on_arrival_and_sells_both_of_its_mana_lines() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[crystal_grotto()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The library before the land arrives: the list's last entry is the top of
    // the library and its first is the bottom, which is the order the scry
    // question reads and the end `ZonePosition::Bottom` writes to.
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    let second = library_before[library_before.len() - 2];

    let grotto = play_land(&mut engine, p0, crystal_grotto());
    assert!(
        engine
            .state()
            .object(grotto)
            .is_some_and(|o| o.zone == Zone::Battlefield),
        "the land is on the battlefield"
    );
    assert!(
        !is_tapped(&engine, grotto),
        "Crystal Grotto prints no entry modifier, so it enters untapped"
    );

    // "When this land enters, scry 1" — the question arrives once the entry
    // trigger has resolved off the stack.
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
    assert_eq!(player, p0, "the seat that played the land does the looking");
    assert_eq!(
        prompt,
        ArrangePrompt::Scry,
        "scry 1 looks at the top card of the library"
    );
    assert_eq!(
        cards,
        vec![top],
        "one card, and it is the top of the library"
    );
    assert_eq!(
        piles,
        scry_piles(1),
        "either the card or nothing may be bottomed"
    );

    engine
        .apply(p0, look_answer(&cards, &[top]))
        .expect("the card the question offered is a legal answer");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert_eq!(
        library.first().copied(),
        Some(top),
        "the chosen card is on the bottom"
    );
    assert_eq!(
        library.last().copied(),
        Some(second),
        "and the card that was under it is the new top"
    );
    assert_eq!(
        library.len(),
        library_before.len(),
        "scry looks and reorders; it draws nothing"
    );

    // "{{T}}: Add {{C}}" — and the empty pool is the whole of the reading:
    // `can_afford` filters the {{1}} line out because there is no mana to pay
    // it with, so exactly one activation of the card is on offer.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let offered: Vec<u32> = legal
        .abilities
        .iter()
        .filter(|(src, _)| *src == grotto)
        .map(|(_, index)| *index)
        .collect();
    assert_eq!(
        offered.len(),
        1,
        "an empty pool pays no {{1}}, so only the colourless line is offered: {offered:?}"
    );
    let colorless = offered[0];
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing is floating on this board"
    );

    activate(&mut engine, p0, crystal_grotto(), colorless);
    assert!(
        !matches!(engine.pending(), Pending::ChooseColor { .. }),
        "`{{C}}` is a fixed colourless and not \"any color\", so nothing is \
         asked on the way: {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "\"{{T}}: Add {{C}}\" — the one thing \"any color\" can never produce"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap, and no mana spent");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    assert!(is_tapped(&engine, grotto), "the Grotto paid its own {{T}}");

    // The second printed line needs a turn: `{T}` is not a price a tapped land
    // has, so the untap step is what stands the Grotto back up for it.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, grotto),
        "the untap step stood the Grotto back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );

    // Two Forests, with the Grotto named as the printing kept back: its own
    // `{T}: Add {C}` is a route `tap_all_mana_but` would otherwise take
    // (#159), and the {{1}} below is read off the pool.
    tap_all_mana_but(&mut engine, p0, Some(crystal_grotto()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests tapped and two green floating"
    );
    assert!(
        !is_tapped(&engine, grotto),
        "and the Grotto was the one thing kept back"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let offered: Vec<u32> = legal
        .abilities
        .iter()
        .filter(|(src, _)| *src == grotto)
        .map(|(_, index)| *index)
        .collect();
    assert_eq!(
        offered.len(),
        2,
        "with a mana floating both printed lines are payable: {offered:?}"
    );
    let any_color = offered
        .iter()
        .copied()
        .find(|index| *index != colorless)
        .expect("the line that is not the colourless one");

    // "{{1}}, {{T}}: Add one mana of any color."
    activate(&mut engine, p0, crystal_grotto(), any_color);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"any color\" is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {options:?}"
    );
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"any color\" includes {color:?}: {options:?}"
        );
    }

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colors it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "one of the two green paid the {{1}}, and the other is still floating"
    );
    assert_eq!(
        pool.total(),
        2,
        "one green and one black, and nothing else: the Forests make green, so \
         the black has no other source on this board"
    );
    assert!(
        is_tapped(&engine, grotto),
        "{{T}} was the other half of the price"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
}
