//! `cards/lands/tapland/ancient_amphitheater.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ancient Amphitheater prints "As this land enters, you may reveal a Giant card from your hand. If you don't, this land enters tapped." and `{{T}}: Add {{R}} or {{W}}.`
///
/// Under `Coverage::Implemented`, the land checks for a Giant card in hand as it enters.
/// Revealing `primeval_titan` satisfies the condition and allows the land to enter untapped while keeping the card in hand, after which activating its mana ability produces the chosen color; without a Giant card in hand, it enters tapped.
#[test]
fn ancient_amphitheater_enters_untapped_by_revealing_giant() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(134, forest())
        .hand(0, &[ancient_amphitheater(), primeval_titan()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, ancient_amphitheater());
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
    let titan = in_hand(&engine, p0, primeval_titan()).expect("titan is in hand");
    assert_eq!(options, vec![titan]);

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![titan],
            },
        )
        .expect("reveal giant card");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, land),
        "revealing a Giant lets Ancient Amphitheater enter untapped"
    );
    assert!(
        in_hand(&engine, p0, primeval_titan()).is_some(),
        "revealed Giant remains in hand"
    );

    activate(&mut engine, p0, ancient_amphitheater(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Red));
    assert!(options.contains(&ManaColor::White));
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));

    let mut engine2 = Duel::new(135, forest())
        .hand(0, &[ancient_amphitheater()])
        .start();
    keep_mulligans(&mut engine2);
    reach_main_phase(&mut engine2, p0);

    let land2 = play_land(&mut engine2, p0, ancient_amphitheater());
    assert!(
        is_tapped(&engine2, land2),
        "with no Giant in hand, Ancient Amphitheater enters tapped"
    );
}
