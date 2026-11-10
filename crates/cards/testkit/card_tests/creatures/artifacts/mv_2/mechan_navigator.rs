//! `cards/creatures/artifacts/mv_2/mechan_navigator.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn mechan_navigator() -> CardIndex {
    card_index("fa72c607-7eb4-41b7-9af1-f3d6c69abf09")
}

/// Mechan Navigator: a 2/1; "Whenever this creature becomes tapped, draw a
/// card, then discard a card." Attacking taps it: the controller draws one,
/// then is asked to discard one and puts the Island in the graveyard.
#[test]
fn mechan_navigator_loots_when_it_becomes_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(&[island()], &[mechan_navigator()], &[]);
    let navigator = on_battlefield(&engine, p0, mechan_navigator()).expect("seated");
    assert_eq!(pt(&engine, navigator), (2, 1));
    let library = library_size(&engine, p0);
    assert_eq!(hand_count(&engine, 0), 1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(navigator, Defender::Player(p1))],
            },
        )
        .expect("the Navigator attacks");
    assert!(is_tapped(&engine, navigator));
    let island_card = in_hand(&engine, p0, island()).expect("the Island is in hand");

    pass_until(&mut engine, |e| {
        !matches!(e.pending(), Pending::Priority { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected a discard question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!(hand_count(&engine, 0), 2, "the card was drawn first");
    assert_eq!(library_size(&engine, p0), library - 1);
    assert!(options.contains(&island_card));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![island_card],
            },
        )
        .expect("discard the Island");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(hand_count(&engine, 0), 1);
    assert!(in_graveyard(&engine, p0, island()).is_some());
}
