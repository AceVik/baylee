//! `cards/lands/gates/izzet_guildgate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Izzet Guildgate prints "This land enters tapped" and "{T}: Add {U} or
/// {R}". Both sentences need a real land drop to be visible: an
/// enters-tapped clause only runs on an actual entry (CR 614.1c), and the
/// mana line is refused while the Gate is still lying down. So the same
/// permanent is read twice — tapped and silent in the turn it arrived,
/// untapped and asking which of the two colours a full turn later — which is
/// the only board that tells a printed restriction from an ability nobody
/// wrote.
#[test]
fn izzet_guildgate_enters_tapped_and_taps_a_turn_later_for_blue_or_red() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(8811, forest())
        .hand(0, &[izzet_guildgate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let gate = play_land(&mut engine, p0, izzet_guildgate());
    assert!(entered_tapped(&engine, gate), "\"This land enters tapped\"");

    // Tapped, so its own {T} is unpayable, and `can_afford` filters the
    // offer off the board rather than off the card text: the ability is not
    // listed at all, in either of the two lists a mana source can live in.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == gate)
            && !legal.mana_abilities.contains(&gate),
        "a tapped Gate has nothing to pay its {{T}} with: {:?}",
        legal.abilities
    );

    // The untap step is what changes the answer, so the walk names the turn
    // rather than assuming one passed.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, gate),
        "its controller's untap step ran (CR 502.3)"
    );

    activate(&mut engine, p0, izzet_guildgate(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"{{U}} or {{R}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Red),
        "the two colours the card prints: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and no third — a Gate adds {{U}} or {{R}} and nothing else: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(pool.available(ManaColor::Blue), 0, "and not the other one");
    assert_eq!(pool.total(), 1, "one mana, off one tap and nothing else");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, gate), "the Gate paid its own {{T}}");
}
