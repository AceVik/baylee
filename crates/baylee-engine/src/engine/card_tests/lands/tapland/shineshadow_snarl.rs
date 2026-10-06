//! `cards/lands/tapland/shineshadow_snarl.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shineshadow Snarl prints "As this land enters, you may reveal a Plains or Swamp card from your hand. If you don't, this land enters tapped." and `{{T}}: Add {{W}} or {{B}}.`
///
/// Under `Coverage::Implemented`, the land checks for a Plains or Swamp card in hand upon entry.
/// Revealing `plains` allows the land to enter untapped while keeping the card in hand, after which activating its mana ability produces the chosen color; without a Plains or Swamp card in hand, it enters tapped.
#[test]
fn shineshadow_snarl_enters_untapped_by_revealing_plains_or_swamp() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(164, forest())
        .hand(0, &[shineshadow_snarl(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, shineshadow_snarl());
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
    let shown = in_hand(&engine, p0, plains()).expect("plains is in hand");
    assert_eq!(options, vec![shown]);

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![shown],
            },
        )
        .expect("reveal plains card");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, land),
        "revealing a Plains lets Shineshadow Snarl enter untapped"
    );
    assert!(
        in_hand(&engine, p0, plains()).is_some(),
        "revealed Plains remains in hand"
    );

    activate(&mut engine, p0, shineshadow_snarl(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::White));
    assert!(options.contains(&ManaColor::Black));
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, land));

    let mut engine2 = Duel::new(165, forest())
        .hand(0, &[shineshadow_snarl()])
        .start();
    keep_mulligans(&mut engine2);
    reach_main_phase(&mut engine2, p0);

    let land2 = play_land(&mut engine2, p0, shineshadow_snarl());
    assert!(
        is_tapped(&engine2, land2),
        "with no Plains or Swamp in hand, Shineshadow Snarl enters tapped"
    );
}
