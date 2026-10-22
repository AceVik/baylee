//! `cards/sorceries/mv_4/lobotomy.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn lobotomy() -> CardIndex {
    card_index("a7ebe748-c3de-402f-8a6e-236a6413c0f8")
}

/// Lobotomy (`a7ebe748-c3de-402f-8a6e-236a6413c0f8`) — {2}{U}{B} Sorcery (Partial):
/// "Target player reveals their hand, then you choose a card other than a basic land card from it..."
#[test]
fn lobotomy_reveals_target_players_hand_and_discards_chosen_non_basic_land() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), swamp(), swamp()])
        .hand(0, &[lobotomy()])
        .hand(1, &[forest(), index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, lobotomy());
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards");
    };
    assert_eq!(options.len(), 1);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
}
