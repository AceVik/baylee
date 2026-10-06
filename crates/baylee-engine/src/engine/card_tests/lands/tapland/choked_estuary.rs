//! `cards/lands/tapland/choked_estuary.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Choked Estuary prints "As this land enters, you may reveal an Island or Swamp card from your hand. If you don't, this land enters tapped." and `{{T}}: Add {{U}} or {{B}}.`
///
/// Under `Coverage::Implemented`, the land offers a reveal of an Island or Swamp card from hand upon entry.
/// Revealing `island` allows the land to enter untapped while keeping the card in hand, after which activating its mana ability produces the chosen color; without an Island or Swamp card in hand, it enters tapped.
#[test]
fn choked_estuary_enters_untapped_by_revealing_island_or_swamp() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(138, forest())
        .hand(0, &[choked_estuary(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, choked_estuary());
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
    let shown = in_hand(&engine, p0, island()).expect("island is in hand");
    assert_eq!(options, vec![shown]);

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![shown],
            },
        )
        .expect("reveal island card");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, land),
        "revealing an Island lets Choked Estuary enter untapped"
    );
    assert!(
        in_hand(&engine, p0, island()).is_some(),
        "revealed Island remains in hand"
    );

    activate(&mut engine, p0, choked_estuary(), 0);
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

    let mut engine2 = Duel::new(139, forest())
        .hand(0, &[choked_estuary()])
        .start();
    keep_mulligans(&mut engine2);
    reach_main_phase(&mut engine2, p0);

    let land2 = play_land(&mut engine2, p0, choked_estuary());
    assert!(
        is_tapped(&engine2, land2),
        "with no Island or Swamp in hand, Choked Estuary enters tapped"
    );
}
