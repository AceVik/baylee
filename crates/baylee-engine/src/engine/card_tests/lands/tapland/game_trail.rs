//! `cards/lands/tapland/game_trail.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Game Trail prints "As this land enters, you may reveal a Mountain or Forest card from your hand. If you don't, this land enters tapped." and `{{T}}: Add {{R}} or {{G}}.`
///
/// Under `Coverage::Implemented`, the land checks for a Mountain or Forest card in hand upon entry.
/// Revealing `mountain` allows the land to enter untapped while keeping the card in hand, after which activating its mana ability produces the chosen color; without a Mountain or Forest card in hand, it enters tapped.
#[test]
fn game_trail_enters_untapped_by_revealing_mountain_or_forest() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(150, island())
        .hand(0, &[game_trail(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, game_trail());
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected reveal prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (0, 1));
    assert_eq!(prompt, ChoicePrompt::RevealOrEnterTapped);
    let shown = in_hand(&engine, p0, mountain()).expect("mountain is in hand");
    assert_eq!(options, vec![shown]);

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![shown],
            },
        )
        .expect("reveal mountain card");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, land),
        "revealing a Mountain lets Game Trail enter untapped"
    );
    assert!(
        in_hand(&engine, p0, mountain()).is_some(),
        "revealed Mountain remains in hand"
    );

    activate(&mut engine, p0, game_trail(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Red));
    assert!(options.contains(&ManaColor::Green));
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));

    let mut engine2 = Duel::new(151, island()).hand(0, &[game_trail()]).start();
    keep_mulligans(&mut engine2);
    reach_main_phase(&mut engine2, p0);

    let land2 = play_land(&mut engine2, p0, game_trail());
    assert!(
        is_tapped(&engine2, land2),
        "with no Mountain or Forest in hand, Game Trail enters tapped"
    );
}
