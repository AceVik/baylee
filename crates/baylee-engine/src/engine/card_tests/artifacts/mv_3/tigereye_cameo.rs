//! `cards/artifacts/mv_3/tigereye_cameo.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tigereye Cameo is `{3}` artifact printing one line: "`{T}`: Add `{G}` or
/// `{W}`." Three Forests pay the cost and leave the pool empty, so the single
/// white that appears afterwards can only have come off the artifact's own
/// tap — and the board holds no white source at all, which is what makes the
/// colour legible instead of assumed. The question the ability asks is two
/// colours wide and has no colourless on it, and the mana arrives with an
/// empty stack because a mana ability resolves as it is activated
/// (CR 605.3b). Silence is the last step: `{W}` is not a label if it cannot
/// pay for a spell that costs it.
#[test]
fn tigereye_cameo_taps_for_green_or_white_and_the_white_pays_a_white_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[tigereye_cameo(), silence()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Three Forests are exactly `{3}`, so the cast spends every mana source on
    // the board and the pool is empty when the artifact lands on it. The
    // Cameo is in hand while the mana is tapped, so the helper cannot have
    // spent the very `{T}` this test activates by hand (#17).
    cast_from_hand(&mut engine, p0, tigereye_cameo());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let cameo = on_battlefield(&engine, p0, tigereye_cameo()).expect("the Cameo resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Forests paid the {{3}} and nothing is left floating"
    );
    assert!(!is_tapped(&engine, cameo), "and the Cameo enters untapped");

    // Ability 0 is the printed "{T}: Add {G} or {W}". A mana ability a card
    // prints has an index to name, so it is an ordinary entry in
    // `legal.abilities` and not the CR 305.6 shortcut.
    activate(&mut engine, p0, tigereye_cameo(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{G}} or {{W}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        2,
        "the two colours the card prints, and no third: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::White),
        "both halves of the printed choice are offered: {options:?}"
    );
    assert!(
        !options.contains(&ManaColor::Colorless),
        "colourless is no colour at all (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white was one of the colours it offered");

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
    assert!(is_tapped(&engine, cameo), "the Cameo paid its own {{T}}");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the one colour that was named, in the pool the moment the tap resolved"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "\"or\" is one mana of one colour: a second green would mean both halves were added"
    );
    assert_eq!(
        pool.total(),
        1,
        "the three Forests are still spent, so one mana is the whole pool"
    );

    // The other half of "this is white": `{W}` in the pool pays for a spell
    // that costs `{W}`, and the pool is empty once it does. A `{G}`
    // mislabelled as white would be refused here.
    cast_with_floating(&mut engine, p0, silence());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, silence()).is_some(),
        "the {{W}} was spent on a white spell rather than sitting in the pool as a label"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and paying for it emptied the pool the Cameo filled"
    );
}
