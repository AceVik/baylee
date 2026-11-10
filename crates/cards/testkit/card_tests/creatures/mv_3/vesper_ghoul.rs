//! `cards/creatures/mv_3/vesper_ghoul.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Vesper Ghoul prints one line: "{T}, Pay 1 life: Add one mana of any
/// color." Neither half of that price is readable out of the card file, so
/// the board is built so each one is visible somewhere a test can look:
/// three Swamps pay the {2}{B} and leave the pool empty, so the single mana
/// that appears afterwards can only have come off the Ghoul's own tap, and
/// the life total falls only because the payment is a cost and not a rider.
/// Naming green is the control — every land on this table makes black, so
/// green in the pool has no other source on it — and the question is five
/// colors wide with no colorless among them (CR 105.4).
#[test]
fn vesper_ghoul_taps_and_pays_a_life_for_one_mana_of_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1177, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[vesper_ghoul()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, vesper_ghoul());
    pass_until(&mut engine, stack_is_empty);
    let ghoul = on_battlefield(&engine, p0, vesper_ghoul()).expect("the Ghoul resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Swamps paid the {{2}}{{B}} and nothing is left floating"
    );
    assert!(!is_tapped(&engine, ghoul), "the Ghoul enters untapped");
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and its price has not been paid yet"
    );

    // A creature cast this turn cannot pay a {T} (CR 302.6), so the ability
    // is read one turn cycle later — the same rule that keeps it out of a
    // combat, asked of a cost instead of an attack.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");

    // The three Swamps are tapped already, so whatever the ability makes is
    // the whole of the pool: no other source on this board is standing.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(ghoul, 0)),
        "the one line the card prints costs no mana, so an empty pool still \
         offers it: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, vesper_ghoul(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` is a question, got {:?}", engine.pending());
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
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the colors it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Black),
        0,
        "every land here makes black and all of them are tapped, so the green \
         has no other source on this board"
    );
    assert_eq!(pool.total(), 1, "one mana, off one activation");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
    assert!(
        is_tapped(&engine, ghoul),
        "the tap symbol is half the price the card prints"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"Pay 1 life\" is the other half, and a cost is paid as the ability \
         is activated"
    );
}
