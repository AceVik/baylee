//! `cards/lands/battle/vernal_fen.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Vernal Fen is a Land — Swamp Forest whose two sentences are "this land
/// enters tapped unless you control two or more basic lands" and
/// "{T}: Add {B} or {G}". Each half gets its own played land: with one basic
/// of your own and three across the table it arrives **tapped**, which is the
/// whole of the count — it is *your* basics, and a tapped `{T}` is not a mana
/// source at all that turn; with a Forest and a Plains of your own it arrives
/// untapped, and the Plains is what keeps "basic land" from being read as
/// "Forest". The land is played rather than seeded onto the battlefield,
/// because `starting_battlefield` places a permanent without an entry and no
/// replacement effect would ever see it.
#[test]
fn vernal_fen_counts_your_own_basics_before_it_lands_and_taps_for_black_or_green() {
    let p0 = PlayerId::new(0);
    let _p1 = PlayerId::new(1);

    // One basic land of p0's own, against three of p1's: four basics stand on
    // the table and only one of them is one this seat controls.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .battlefield(1, &[forest(), forest(), forest()])
        .hand(0, &[vernal_fen()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let fen = play_land(&mut engine, p0, vernal_fen());
    assert!(
        entered_tapped(&engine, fen),
        "one basic land you control is not two, and the three across the table \
         are not yours"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a land drop hands priority straight back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.mana_abilities.contains(&fen)
            && !legal.abilities.iter().any(|(source, _)| *source == fen),
        "and a tapped `{{T}}` is no mana source, so arriving tapped costs it the \
         whole turn: {legal:?}"
    );

    // The other side of the same count, and the colour choice that is the
    // card's only other sentence: two basics of your own, one of them a Plains
    // so that "basic land" is read and not "Forest".
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), plains()])
        .hand(0, &[vernal_fen()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let fen = play_land(&mut engine, p0, vernal_fen());
    assert!(
        !entered_tapped(&engine, fen),
        "two basic lands you control is two, and a Forest beside a Plains is two"
    );

    activate(&mut engine, p0, vernal_fen(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`{{T}}: Add {{B}} or {{G}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped the land names the colour");
    assert!(
        options.contains(&ManaColor::Black) && options.contains(&ManaColor::Green),
        "a Swamp Forest makes either of the two it prints: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and those two are the whole menu — no third colour and not the \
         colourless that neither a Swamp nor a Forest prints: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the one of the two it was not asked for"
    );
    assert_eq!(pool.total(), 1, "one land, one tap, one mana");
    assert!(is_tapped(&engine, fen), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
}
