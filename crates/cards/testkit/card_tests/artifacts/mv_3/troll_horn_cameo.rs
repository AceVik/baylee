//! `cards/artifacts/mv_3/troll_horn_cameo.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Troll-Horn Cameo is a `{3}` artifact printing one line: `{T}: Add {R} or
/// {G}`. The whole card is the *choice*, so the reading worth playing is the
/// question itself — a `Pending::ChooseColor` two options wide, with exactly
/// the two colours the card names and neither of the other three. The {3} is
/// a real payment: three Forests are spent down to an empty pool, so the one
/// mana left floating afterwards can only have come off the artifact's own
/// tap, and the green that was never named is what tells "or" from "and"
/// (CR 605.3b keeps the whole thing off the stack).
#[test]
fn troll_horn_cameo_taps_for_red_or_green_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(911, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[troll_horn_cameo()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The {3} is paid out of three Forests and leaves nothing behind, which
    // is what makes the pool below a claim about the Cameo and not about a
    // land that happened to be tapped for it.
    cast_from_hand(&mut engine, p0, troll_horn_cameo());
    pass_until(&mut engine, stack_is_empty);
    let cameo = on_battlefield(&engine, p0, troll_horn_cameo()).expect("the Cameo resolved");
    assert!(!is_tapped(&engine, cameo), "it enters untapped and ready");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the three Forests paid the {{3}} to the last mana"
    );

    // Ability 0 is the printed "{T}: Add {R} or {G}", and `{T}` is the only
    // price it costs, so it is offered without a single mana floating.
    activate(&mut engine, p0, troll_horn_cameo(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`{{R}} or {{G}}` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert!(
        options.contains(&ManaColor::Red) && options.contains(&ManaColor::Green),
        "both halves of `or` are on the menu: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and neither of the other three colours is: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was one of the colours it offered");

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, cameo), "the Cameo paid its own {{T}}");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "`or` is one colour: the other half of the menu was not added beside it"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
}
