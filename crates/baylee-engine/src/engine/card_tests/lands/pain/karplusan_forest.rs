//! `cards/lands/pain/karplusan_forest.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Karplusan Forest prints **two** mana abilities where a land usually prints
/// one: `{T}: Add {C}`, and `{T}: Add {R} or {G}. This land deals 1 damage to
/// you.` The split is the whole card, so one tap reading one mana says nothing
/// — the damage is the only thing that ever tells the second sentence from the
/// first, and the `ChooseColor` menu is the only thing that says the coloured
/// sentence offers two colours and not the `{C}` beside it. Both halves are
/// pressed on the same permanent a turn apart, because a land taps once.
#[test]
fn karplusan_forest_taps_for_colourless_for_free_and_for_red_or_green_at_one_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[karplusan_forest()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, karplusan_forest());
    assert!(
        !is_tapped(&engine, land),
        "the card prints no enter modifier, so the land stands the moment it \
         is played and its {{T}} is payable this turn"
    );

    // Ability 0: "{T}: Add {C}." No colour is asked for and no life is paid.
    activate(&mut engine, p0, karplusan_forest(), 0);
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to \
         resolve"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "the colourless sentence made a colourless mana"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and nothing else came with it"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "this half of the card deals no damage, which is what the life total \
         is read for below"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");

    // A land taps once, so the coloured half waits for its own untap step.
    // The pool is empty on the far side regardless: it empties when a step
    // or phase ends (CR 500.5).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "the untap step stood it back up");

    // Ability 1: "{T}: Add {R} or {G}. This land deals 1 damage to you."
    activate(&mut engine, p0, karplusan_forest(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{R}} or {{G}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names it");
    assert_eq!(
        options,
        vec![ManaColor::Red, ManaColor::Green],
        "the two colours the second sentence prints — and not the {{C}} the \
         first one adds, which would mean the two abilities were one"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the two colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Colorless),
        0,
        "one tap, one mana: the two sentences do not share a pool entry"
    );
    assert_eq!(pool.total(), 1, "and nothing beside it");
    assert!(
        stack_is_empty(&engine),
        "a mana ability with a rider is still a mana ability (CR 605.1a), so \
         it never uses the stack either"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"This land deals 1 damage to you\" — charged to the seat that \
         tapped it, where the {{C}} ability above charged nothing"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and never to the opponent"
    );
    assert!(is_tapped(&engine, land), "the second {{T}} was paid too");
}
