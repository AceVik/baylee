//! `cards/lands/gain/rugged_highlands.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rugged Highlands prints three lines and one land drop reads all three: it
/// enters tapped, its arrival gains its controller 1 life, and it taps for
/// {R} or {G}. The life is read off both seats, so the point is the
/// controller's and not the table's, and the mana line is read twice over the
/// same permanent — once on the turn it arrived, where the {T} it costs is
/// already spent and the line is offered by neither list, and once after the
/// untap step that gives it back, where the offer is a *choice* between
/// exactly its two printed colours.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn rugged_highlands_enters_tapped_gains_a_life_and_taps_for_red_or_green() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(7712, forest())
        .hand(0, &[rugged_highlands()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, rugged_highlands());
    assert!(
        is_tapped(&engine, land),
        "\"This land enters tapped\": the status is on the permanent as it lands"
    );

    // The control for the offer read after the untap step below. This is the
    // same permanent one turn earlier, and the {T} its mana line costs cannot
    // be paid, so the line is offered by neither of the two lists an offer
    // can live in.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a land drop leaves the seat holding priority, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == land)
            && !legal.mana_abilities.contains(&land),
        "a tapped permanent pays no {{T}}: {:?} / {:?}",
        legal.abilities,
        legal.mana_abilities
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        21,
        "\"When this land enters, you gain 1 life\""
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the life is the land's controller's, not the table's"
    );
    assert!(
        is_tapped(&engine, land),
        "the enter trigger resolving does not stand the land back up"
    );

    // Across a whole turn and back: only the untap step of its own controller
    // can undo the printed entry, and it is also what makes the mana line
    // payable again.
    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > turn
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == p0
    });
    assert!(
        !is_tapped(&engine, land),
        "the untap step of p0's own turn stands it back up (CR 502.3)"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a main phase hands priority back, got {:?}",
            engine.pending()
        )
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == land)
        .expect("the untapped land prints a mana ability, and now it is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("its whole price is its own {{T}}");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{R}} or {{G}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that taps the land names the colour");
    assert!(
        options.contains(&ManaColor::Red) && options.contains(&ManaColor::Green),
        "\"Add {{R}} or {{G}}\" offers both: {options:?}"
    );
    assert_eq!(options.len(), 2, "exactly the two it prints: {options:?}");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the two it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(pool.total(), 1, "one tap, one mana");
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "and the other colour was not made as well"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
}
