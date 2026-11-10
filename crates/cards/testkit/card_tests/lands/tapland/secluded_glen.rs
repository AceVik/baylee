//! `cards/lands/tapland/secluded_glen.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Secluded Glen prints "As this land enters, you may reveal a Faerie card from your hand. If you don't, this land enters tapped." and `{{T}}: Add {{U}} or {{B}}.`
///
/// Under `Coverage::Implemented`, the land checks for a Faerie card in hand upon entry.
/// Revealing `vendilion_clique` allows the land to enter untapped while keeping the card in hand, after which activating its mana ability produces the chosen color; without a Faerie card in hand, it enters tapped.
#[test]
fn secluded_glen_enters_untapped_by_revealing_faerie() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(162, forest())
        .hand(0, &[secluded_glen(), vendilion_clique()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, secluded_glen());
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
    let faerie = in_hand(&engine, p0, vendilion_clique()).expect("clique is in hand");
    assert_eq!(options, vec![faerie]);

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![faerie],
            },
        )
        .expect("reveal faerie card");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, land),
        "revealing a Faerie lets Secluded Glen enter untapped"
    );
    assert!(
        in_hand(&engine, p0, vendilion_clique()).is_some(),
        "revealed Faerie remains in hand"
    );

    activate(&mut engine, p0, secluded_glen(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Blue));
    assert!(options.contains(&ManaColor::Black));
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, land));

    let mut engine2 = Duel::new(163, forest()).hand(0, &[secluded_glen()]).start();
    keep_mulligans(&mut engine2);
    reach_main_phase(&mut engine2, p0);

    let land2 = play_land(&mut engine2, p0, secluded_glen());
    assert!(
        is_tapped(&engine2, land2),
        "with no Faerie in hand, Secluded Glen enters tapped"
    );
}
