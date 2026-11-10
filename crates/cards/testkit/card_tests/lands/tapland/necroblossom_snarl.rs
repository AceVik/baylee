//! `cards/lands/tapland/necroblossom_snarl.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Necroblossom Snarl prints "As this land enters, you may reveal a Swamp or Forest card from your hand. If you don't, this land enters tapped." and `{{T}}: Add {{B}} or {{G}}.`
///
/// Under `Coverage::Implemented`, the land checks for a Swamp or Forest card in hand upon entry.
/// Revealing `swamp` allows the land to enter untapped while keeping the card in hand, after which activating its mana ability produces the chosen color; without a Swamp or Forest card in hand, it enters tapped.
#[test]
fn necroblossom_snarl_enters_untapped_by_revealing_swamp_or_forest() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(156, island())
        .hand(0, &[necroblossom_snarl(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, necroblossom_snarl());
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
    let shown = in_hand(&engine, p0, swamp()).expect("swamp is in hand");
    assert_eq!(options, vec![shown]);

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![shown],
            },
        )
        .expect("reveal swamp card");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, land),
        "revealing a Swamp lets Necroblossom Snarl enter untapped"
    );
    assert!(
        in_hand(&engine, p0, swamp()).is_some(),
        "revealed Swamp remains in hand"
    );

    activate(&mut engine, p0, necroblossom_snarl(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Black));
    assert!(options.contains(&ManaColor::Green));
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, land));

    let mut engine2 = Duel::new(157, island())
        .hand(0, &[necroblossom_snarl()])
        .start();
    keep_mulligans(&mut engine2);
    reach_main_phase(&mut engine2, p0);

    let land2 = play_land(&mut engine2, p0, necroblossom_snarl());
    assert!(
        is_tapped(&engine2, land2),
        "with no Swamp or Forest in hand, Necroblossom Snarl enters tapped"
    );
}
