//! `cards/lands/tapland/wanderwine_hub.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wanderwine Hub prints "As this land enters, you may reveal a Merfolk card from your hand. If you don't, this land enters tapped." and `{{T}}: Add {{W}} or {{U}}.`
///
/// Under `Coverage::Implemented`, the land checks for a Merfolk card in hand upon entry.
/// Revealing `world_shaper` allows the land to enter untapped while keeping the card in hand, after which activating its mana ability produces the chosen color; without a Merfolk card in hand, it enters tapped.
#[test]
fn wanderwine_hub_enters_untapped_by_revealing_merfolk() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(168, forest())
        .hand(0, &[wanderwine_hub(), world_shaper()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, wanderwine_hub());
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
    let merfolk = in_hand(&engine, p0, world_shaper()).expect("world shaper is in hand");
    assert_eq!(options, vec![merfolk]);

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![merfolk],
            },
        )
        .expect("reveal merfolk card");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, land),
        "revealing a Merfolk lets Wanderwine Hub enter untapped"
    );
    assert!(
        in_hand(&engine, p0, world_shaper()).is_some(),
        "revealed Merfolk remains in hand"
    );

    activate(&mut engine, p0, wanderwine_hub(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::White));
    assert!(options.contains(&ManaColor::Blue));
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, land));

    let mut engine2 = Duel::new(169, forest())
        .hand(0, &[wanderwine_hub()])
        .start();
    keep_mulligans(&mut engine2);
    reach_main_phase(&mut engine2, p0);

    let land2 = play_land(&mut engine2, p0, wanderwine_hub());
    assert!(
        is_tapped(&engine2, land2),
        "with no Merfolk in hand, Wanderwine Hub enters tapped"
    );
}
