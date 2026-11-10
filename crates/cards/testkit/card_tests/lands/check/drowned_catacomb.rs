//! `cards/lands/check/drowned_catacomb.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Drowned Catacomb: "This land enters tapped unless you control an Island
/// or a Swamp" and "{T}: Add {U} or {B}."
///
/// The condition is played against three boards, because each one rules out
/// a different way of being wrong: an Island of your own is the untapped
/// branch, a Swamp of your own is the other half of the printed `or`, and a
/// board whose only Island belongs to the **opponent** — with a Forest of
/// yours beside it — turns tapped only if both `ControlledByYou` and the
/// subtype test were actually read. The first board then taps, which is the
/// only way to see that `{U} or {B}` is a question with exactly two answers
/// and that the blue in the pool came off the Catacomb (nothing else on the
/// board was pressed, so the untapped Island beside it cannot account for
/// the mana).
#[test]
fn drowned_catacomb_checks_your_own_island_or_swamp_then_taps_for_blue_or_black() {
    let p0 = PlayerId::new(0);
    let _p1 = PlayerId::new(1);

    // An Island of your own: the condition holds and the land arrives
    // untapped, which is also the board its mana ability is read off.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island()])
        .hand(0, &[drowned_catacomb()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let catacomb = play_land(&mut engine, p0, drowned_catacomb());
    assert!(
        !entered_tapped(&engine, catacomb),
        "\"enters tapped unless you control an Island or a Swamp\" — the \
         Island is right there"
    );

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing is floating and nothing has been tapped before the Catacomb acts"
    );
    // `{T}: Add {U} or {B}` is a *printed* mana ability, so it is an ordinary
    // `(source, index)` entry rather than a CR 305.6 shortcut, and it asks
    // which of the two colours it is making.
    activate(&mut engine, p0, drowned_catacomb(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`{{U}} or {{B}}` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert_eq!(
        options.len(),
        2,
        "two colours and no third — colourless is no colour at all (CR 105.4): {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Black),
        "blue and black, in whichever order the engine lists them: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the two it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana off one tap: the untapped Island beside it was never \
         pressed, so this blue has no other source on the board"
    );
    assert!(
        is_tapped(&engine, catacomb),
        "and the Catacomb paid its own {{T}} to make it"
    );

    // A Swamp of your own: the other half of the printed `or`, and the board
    // that stays tapped if the check was written for Islands alone.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp()])
        .hand(0, &[drowned_catacomb()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let catacomb = play_land(&mut engine, p0, drowned_catacomb());
    assert!(
        !entered_tapped(&engine, catacomb),
        "\"an Island **or a Swamp**\": the Swamp is enough on its own"
    );

    // The Island that is not yours, with a Forest of yours beside it. Both
    // readings of the filter are struck at once: a check that counted any
    // Island on the battlefield, and one that counted your lands without
    // looking at their types, each leave this arrival untapped.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .battlefield(1, &[island()])
        .hand(0, &[drowned_catacomb()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let catacomb = play_land(&mut engine, p0, drowned_catacomb());
    assert!(
        entered_tapped(&engine, catacomb),
        "the only Island on the battlefield belongs to the opponent, and the \
         Forest under your own control is neither an Island nor a Swamp: \
         \"you control\" and the subtype are both read"
    );
}
