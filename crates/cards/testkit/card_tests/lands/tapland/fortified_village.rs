//! `cards/lands/tapland/fortified_village.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fortified Village prints "As this land enters, you may reveal a Forest or Plains card from your hand. If you don't, this land enters tapped." and `{{T}}: Add {{G}} or {{W}}.`
///
/// Under `Coverage::Implemented`, the land checks for a Forest or Plains card in hand upon entry.
/// Revealing `plains` allows the land to enter untapped while keeping the card in hand, after which activating its mana ability produces the chosen color; without a Forest or Plains card in hand, it enters tapped.
#[test]
fn fortified_village_enters_untapped_by_revealing_forest_or_plains() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(144, island())
        .hand(0, &[fortified_village(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, fortified_village());
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
        "revealing a Plains lets Fortified Village enter untapped"
    );
    assert!(
        in_hand(&engine, p0, plains()).is_some(),
        "revealed Plains remains in hand"
    );

    activate(&mut engine, p0, fortified_village(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Green));
    assert!(options.contains(&ManaColor::White));
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));

    let mut engine2 = Duel::new(145, island())
        .hand(0, &[fortified_village()])
        .start();
    keep_mulligans(&mut engine2);
    reach_main_phase(&mut engine2, p0);

    let land2 = play_land(&mut engine2, p0, fortified_village());
    assert!(
        is_tapped(&engine2, land2),
        "with no Forest or Plains in hand, Fortified Village enters tapped"
    );
}
