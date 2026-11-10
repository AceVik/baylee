//! `cards/lands/tapland/auntie_s_hovel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Auntie's Hovel prints "As this land enters, you may reveal a Goblin card from your hand. If you don't, this land enters tapped." and `{{T}}: Add {{B}} or {{R}}.`
///
/// Under `Coverage::Implemented`, the land checks for a Goblin card in hand upon entry.
/// Revealing `festering_goblin` satisfies the condition and allows the land to enter untapped while keeping the card in hand, after which activating its mana ability produces the chosen color; without a Goblin card in hand, it enters tapped.
#[test]
fn auntie_s_hovel_enters_untapped_by_revealing_goblin() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(136, forest())
        .hand(0, &[auntie_s_hovel(), festering_goblin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, auntie_s_hovel());
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
    let goblin = in_hand(&engine, p0, festering_goblin()).expect("goblin is in hand");
    assert_eq!(options, vec![goblin]);

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![goblin],
            },
        )
        .expect("reveal goblin card");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, land),
        "revealing a Goblin lets Auntie's Hovel enter untapped"
    );
    assert!(
        in_hand(&engine, p0, festering_goblin()).is_some(),
        "revealed Goblin remains in hand"
    );

    activate(&mut engine, p0, auntie_s_hovel(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Black));
    assert!(options.contains(&ManaColor::Red));
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, land));

    let mut engine2 = Duel::new(137, forest())
        .hand(0, &[auntie_s_hovel()])
        .start();
    keep_mulligans(&mut engine2);
    reach_main_phase(&mut engine2, p0);

    let land2 = play_land(&mut engine2, p0, auntie_s_hovel());
    assert!(
        is_tapped(&engine2, land2),
        "with no Goblin in hand, Auntie's Hovel enters tapped"
    );
}
