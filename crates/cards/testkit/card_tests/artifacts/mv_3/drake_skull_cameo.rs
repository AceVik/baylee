//! `cards/artifacts/mv_3/drake_skull_cameo.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Drake-Skull Cameo is a {3} artifact printing one line: "{T}: Add {U} or
/// {B}." The word that carries the card is "or", so the test reads the
/// question the engine asks and *both* of its answers rather than the mana
/// alone: a permanent that only ever made blue would ask nothing at all, and
/// one that made both colours per tap would be a different card.
///
/// The three Forests are what makes every reading exact. They pay the {3}, so
/// the pool is empty the moment the artifact lands and whatever is in it
/// afterwards came off the Cameo's own tap — and they are tapped, so nothing
/// else on this board could have produced blue.
#[test]
fn drake_skull_cameo_taps_for_blue_or_black_and_offers_both_halves_of_the_choice() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[drake_skull_cameo()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {3} off the three Forests, which leaves the pool empty: the mana read
    // below is the Cameo's own and not a land's that happened to be untapped.
    cast_from_hand(&mut engine, p0, drake_skull_cameo());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let cameo = on_battlefield(&engine, p0, drake_skull_cameo()).expect("the Cameo resolved");
    assert!(
        types(&engine, cameo).contains(TypeSet::ARTIFACT),
        "it is the artifact it prints"
    );
    assert!(!is_tapped(&engine, cameo), "an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Forests paid the {{3}} and left nothing floating"
    );

    // Ability 0 is the printed "{T}: Add {U} or {B}" — a mana ability a card
    // prints, so it is an ordinary entry in `abilities` with an index to name
    // rather than the CR 305.6 shortcut, and its whole price is its own tap.
    activate(&mut engine, p0, drake_skull_cameo(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{U}} or {{B}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        2,
        "the two colours the card prints and no third: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Blue),
        "blue is one half of the choice, and the creature in the name's \
         colour it is: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Black),
        "and black the other half — a printing with only one of them would \
         ask nothing and offer one: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Black),
        0,
        "`or` is exclusive: naming one half of the choice is not making both"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the three Forests were spent on the {{3}} and make green besides, so \
         the blue has no other source on this board"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
    assert!(is_tapped(&engine, cameo), "the Cameo paid its own {{T}}");
}
