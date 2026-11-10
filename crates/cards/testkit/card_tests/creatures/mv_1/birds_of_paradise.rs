//! `cards/creatures/mv_1/birds_of_paradise.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Birds of Paradise prints three things: a 0/1 Bird body, flying, and
/// "{T}: Add one mana of any color." The board holds the Bird and nothing
/// else, so whatever reaches the pool got there off its own tap — and the
/// color named is black, which no Forest in this deck could have produced.
/// "Any color" is a question (CR 105.4), so the five colors are read off the
/// prompt itself rather than assumed, and the tap is read off the Bird
/// afterwards.
#[test]
fn birds_of_paradise_taps_for_one_mana_of_the_color_its_controller_names() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[birds_of_paradise()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let birds = on_battlefield(&engine, p0, birds_of_paradise()).expect("the Bird is out");
    assert_eq!(pt(&engine, birds), (0, 1), "the body the card prints");
    assert!(
        keywords(&engine, birds).contains(KeywordSet::FLYING),
        "the printed flying line"
    );
    assert!(!is_tapped(&engine, birds), "nothing has tapped it yet");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool is empty before the Bird is asked"
    );

    // The only permanent on this side is the Bird, so whatever lands in the
    // pool landed there off its own {T}.
    activate(&mut engine, p0, birds_of_paradise(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"any color\" is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat that activated names the color");
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
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colors it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, birds), "the Bird paid its own {{T}}");
}
