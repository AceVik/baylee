//! `cards/lands/tapland/gilt_leaf_palace.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gilt-Leaf Palace prints "As this land enters, you may reveal an Elf card from your hand. If you don't, this land enters tapped." and `{{T}}: Add {{B}} or {{G}}.`
///
/// Under `Coverage::Implemented`, the land checks for an Elf card in hand upon entry.
/// Revealing `llanowar_elves` allows the land to enter untapped while keeping the card in hand, after which activating its mana ability produces the chosen color; without an Elf card in hand, it enters tapped.
#[test]
fn gilt_leaf_palace_enters_untapped_by_revealing_elf() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(152, island())
        .hand(0, &[gilt_leaf_palace(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, gilt_leaf_palace());
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
    let elf = in_hand(&engine, p0, llanowar_elves()).expect("elf is in hand");
    assert_eq!(options, vec![elf]);

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("reveal elf card");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, land),
        "revealing an Elf lets Gilt-Leaf Palace enter untapped"
    );
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_some(),
        "revealed Elf remains in hand"
    );

    activate(&mut engine, p0, gilt_leaf_palace(), 0);
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

    let mut engine2 = Duel::new(153, island())
        .hand(0, &[gilt_leaf_palace()])
        .start();
    keep_mulligans(&mut engine2);
    reach_main_phase(&mut engine2, p0);

    let land2 = play_land(&mut engine2, p0, gilt_leaf_palace());
    assert!(
        is_tapped(&engine2, land2),
        "with no Elf in hand, Gilt-Leaf Palace enters tapped"
    );
}
