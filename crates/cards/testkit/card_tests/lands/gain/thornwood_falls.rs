//! `cards/lands/gain/thornwood_falls.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Thornwood Falls prints three lines: it enters tapped, its controller gains
/// 1 life as it enters, and it taps for `{G}` or `{U}`. The land is *played*
/// and not seeded onto the battlefield, because `starting_battlefield` places
/// a permanent without an entry — no replacement effect looks at it — so a
/// board built that way would arrive untapped and gain no life and both entry
/// clauses would read as missing. That same tapped arrival is the control for
/// the mana line: while the land is down its `{T}` is not on the offer at
/// all, and only its controller's untap step hands the ability back. The
/// colour question is read off the pool rather than off the board, so nothing
/// is floating before the tap and the one blue afterwards has no other source
/// on the table.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn thornwood_falls_arrives_tapped_pays_a_life_and_taps_for_green_or_blue() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[thornwood_falls()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let falls = play_land(&mut engine, p0, thornwood_falls());
    assert!(
        is_tapped(&engine, falls),
        "\"This land enters tapped\" — a land that was played, not placed"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        21,
        "20 from the format and 1 from \"when this land enters, you gain 1 life\""
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the life goes to the land's controller and nobody else"
    );

    // The other half of the entry clause, and the reason the untap step below
    // is load-bearing: a land tapped by its own entry has no {T} to pay with.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the trigger resolved back into a quiet main phase, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == falls),
        "a land still tapped from its own entry may not be tapped for mana: {:?}",
        legal.abilities
    );

    // One turn cycle: the untap step is what stands the land back up.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, falls),
        "the untap step ran and the land came back"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "p0's own main phase offers priority, got {:?}",
            engine.pending()
        )
    };
    let routes: Vec<(ObjectId, u32)> = legal
        .abilities
        .iter()
        .copied()
        .filter(|(src, _)| *src == falls)
        .collect();
    assert_eq!(
        routes.len(),
        1,
        "the only activated ability the land prints: {routes:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing is floating, so the mana read below came off this tap alone"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: routes[0].0,
                ability_index: routes[0].1,
            },
        )
        .unwrap();

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{G}} or {{U}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        2,
        "two colours printed, and colourless is no colour at all \
         (CR 105.4): {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::Blue),
        "{{G}} and {{U}} and nothing else: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the two colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "and not the other colour it could have made"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, falls), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
}
