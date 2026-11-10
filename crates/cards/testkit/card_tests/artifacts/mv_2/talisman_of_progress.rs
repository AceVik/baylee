//! `cards/artifacts/mv_2/talisman_of_progress.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Talisman of Progress prints two mana abilities — "`{T}`: Add `{C}`" and
/// "`{T}`: Add `{W}` or `{U}`. This artifact deals 1 damage to you." — and
/// the second one is the whole card, because its two halves land in two
/// different places. The menu is exactly the two colours the card prints and
/// has no colourless on it (CR 105.4), which is what tells this line from the
/// `{T}: Add {C}` above it; and the single damage is read on *both* seats,
/// since "to you" is the word under test and a card that had aimed it across
/// the table would leave the controller at twenty. Both lines cost no mana at
/// all, so the pool is empty before the tap and everything in it afterwards
/// came off the artifact.
#[test]
fn talisman_of_progress_taps_for_one_of_its_two_colors_and_bites_its_controller() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[talisman_of_progress()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let talisman =
        on_battlefield(&engine, p0, talisman_of_progress()).expect("the Talisman is on the table");
    assert!(!is_tapped(&engine, talisman), "it starts untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing is floating on a board whose only permanent is the Talisman"
    );

    // The whole price of both printed lines is the tap symbol, so both are
    // offered before anything is spent.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(talisman, 0)),
        "{{T}}: Add {{C}} is offered: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(talisman, 1)),
        "{{T}}: Add {{W}} or {{U}} is offered beside it: {:?}",
        legal.abilities
    );

    // Ability 1 is the coloured one; ability 0 is the colourless tap.
    activate(&mut engine, p0, talisman_of_progress(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`{{W}} or {{U}}` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options,
        vec![ManaColor::White, ManaColor::Blue],
        "the two colours the card prints, and colourless is no colour at all \
         (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana and the damage \
         are already here"
    );
    assert!(is_tapped(&engine, talisman), "{{T}} was the price");
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"This artifact deals 1 damage to you\""
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the damage belongs to the controller, not to the opponent"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::White),
        0,
        "the other half of the choice was not paid for as well"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
}
