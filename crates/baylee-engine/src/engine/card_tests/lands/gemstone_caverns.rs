//! `cards/lands/gemstone_caverns.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::YesNoPrompt;

/// Gemstone Caverns:
/// - "If this card is in your opening hand and you're not the starting player,
///   you may begin the game with Gemstone Caverns on the battlefield with a
///   luck counter on it. If you do, exile a card from your hand."
/// - "{T}: Add {C}. If Gemstone Caverns has a luck counter on it, instead add
///   one mana of any color."
#[test]
fn gemstone_caverns_begins_on_battlefield_with_luck_counter_for_non_starting_player_and_adds_any_color()
 {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(19, forest())
        .hand(1, &[gemstone_caverns(), forest()])
        .start();

    // Both players keep their opening hands; p0 is the starting player, so p1
    // is offered the Gemstone Caverns opening-hand action (CR 103.6).
    engine.apply(p0, PlayerAction::MulliganKeep).unwrap();
    engine.apply(p1, PlayerAction::MulliganKeep).unwrap();

    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::Generic,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected opening-hand YesNo prompt for p1, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p1);
    engine.apply(p1, PlayerAction::YesNo(true)).unwrap();

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected ExileFromHand ChooseCards prompt for p1, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p1);
    assert_eq!((min, max), (1, 1));
    let pitch = options[0];
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![pitch],
            },
        )
        .unwrap();

    let caverns = on_battlefield(&engine, p1, gemstone_caverns())
        .expect("Gemstone Caverns began the game on the battlefield");
    assert_eq!(
        counters_on(&engine, caverns, counters::LUCK),
        1,
        "Gemstone Caverns entered with a luck counter"
    );
    assert_eq!(
        engine.state().object(pitch).expect("pitched card").zone,
        Zone::Exile,
        "the chosen card from p1's hand was exiled"
    );

    // On p1's main phase, tapping Gemstone Caverns with a luck counter adds
    // one mana of any color.
    reach_their_main_phase(&mut engine, p1);
    activate(&mut engine, p1, gemstone_caverns(), 0);
    let Pending::ChooseColor { player, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseColor with luck counter, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p1);
    engine
        .apply(p1, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );
}

#[test]
fn gemstone_caverns_played_normally_without_luck_counter_taps_for_colourless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(19, forest())
        .hand(0, &[gemstone_caverns()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let card = in_hand(&engine, p0, gemstone_caverns()).expect("the Caverns are in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card })
        .expect("a land drop on an empty board");
    let caverns = on_battlefield(&engine, p0, gemstone_caverns()).expect("the land hit the table");
    assert_eq!(counters_on(&engine, caverns, counters::LUCK), 0);

    activate(&mut engine, p0, gemstone_caverns(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, caverns));
}
