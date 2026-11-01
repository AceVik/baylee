//! `cards/instants/mv_3/intuition.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn intuition() -> CardIndex {
    card_index("3c9faba7-f2d3-4978-be94-020dc8003dc0")
}

/// `Intuition` ({2}{U}, Instant, `Coverage::Partial`):
/// `Search your library for three cards and reveal them. Target opponent
/// chooses one. Put that card into your hand and the rest into your graveyard.
/// Then shuffle.`
#[test]
fn intuition_searches_for_three_cards_and_splits_two_to_graveyard_and_one_to_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[intuition(), plains(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    hide_in_library(&mut engine, p0, &[plains(), swamp()]);

    cast_from_hand(&mut engine, p0, intuition());
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected library search choice");
    };
    let found = options[..3].to_vec();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: found.clone(),
            },
        )
        .unwrap();

    let Pending::ChooseCards { player, .. } = engine.pending().clone() else {
        panic!("expected opponent split choice");
    };
    assert_eq!(player, p1);
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: found[..2].to_vec(),
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(found[0]).map(|o| o.zone),
        Some(crate::zone::Zone::Graveyard)
    );
    assert_eq!(
        engine.state().object(found[1]).map(|o| o.zone),
        Some(crate::zone::Zone::Graveyard)
    );
    assert_eq!(
        engine.state().object(found[2]).map(|o| o.zone),
        Some(crate::zone::Zone::Hand)
    );
}
