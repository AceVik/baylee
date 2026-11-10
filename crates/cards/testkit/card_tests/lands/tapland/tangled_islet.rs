//! `cards/lands/tapland/tangled_islet.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tangled Islet is a `Forest Island` that "enters tapped" and taps for `{G}`
/// or `{U}`.
///
/// The entry is played as a real land drop, because `starting_battlefield`
/// places a permanent with `Cause::Setup` and no replacement effect ever looks
/// at it — a board built that way arrives untapped whatever the card prints.
/// The Forest beside it is the control for the turn the islet spends tapped:
/// the Forest *is* offered, so the islet's absence from both lists is the tap
/// and not the phase.
///
/// The second half needs an untap step, so the game walks a full turn cycle —
/// `reach_their_main_phase` crosses the combat the first turn asks about. A
/// Forest Island then taps for either of its two basic land types, and the
/// colour question the engine asks is what says so: the blue in the pool
/// afterwards cannot have come off the Forest, which makes green and never
/// moved.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn tangled_islet_enters_tapped_and_untaps_into_a_forest_island() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9317, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[tangled_islet()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let islet = play_land(&mut engine, p0, tangled_islet());
    assert!(
        entered_tapped(&engine, islet),
        "the printed line: This land enters tapped"
    );
    assert!(
        types(&engine, islet).contains(TypeSet::LAND),
        "and what arrived is a land"
    );

    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is on the battlefield");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.mana_abilities.contains(&land),
        "an untapped Forest is a mana source, so the offer is not empty: {:?}",
        legal.mana_abilities
    );
    assert!(
        !legal.mana_abilities.contains(&islet)
            && !legal.abilities.iter().any(|(src, _)| *src == islet),
        "a land that entered tapped has no {{T}} to pay with until its \
         controller's untap step, whichever list its mana route lives in: {:?} / {:?}",
        legal.mana_abilities,
        legal.abilities
    );

    // Across the opponent's turn and back: `reach_their_main_phase` is
    // `pass_until` with a phase predicate, so it survives the attack
    // declaration `reach_main_phase` would panic on.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, islet),
        "the untap step stands it back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing has been tapped for mana yet, so the pool read below is \
         the islet's alone"
    );

    // Which list carries the route is the engine's business: a land with a
    // basic land type is the CR 305.6 shortcut named in `mana_abilities`,
    // while a mana ability the card *prints* is an ordinary indexed entry
    // (#159). The test takes whichever it was offered.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let route = if legal.mana_abilities.contains(&islet) {
        PlayerAction::ActivateManaAbility { source: islet }
    } else {
        let (source, ability_index) = legal
            .abilities
            .iter()
            .copied()
            .find(|(src, _)| *src == islet)
            .expect("an untapped Forest Island offers a way to make mana");
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        }
    };
    engine
        .apply(p0, route)
        .expect("the offer named this route itself");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{G}} or {{U}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that activated names the colour");
    assert!(
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::Blue),
        "both of the printed basic land types are on the menu: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and nothing else is — a Forest Island makes green or blue: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the two types it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "one tap pays for one colour, not for both types at once"
    );
    assert_eq!(
        pool.total(),
        1,
        "exactly one mana, and the Forest beside it contributed none of it"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, islet), "the islet paid its own {{T}}");
    assert!(
        !is_tapped(&engine, land),
        "and the Forest never moved, so the blue has no other source on this board"
    );
}
