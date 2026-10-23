//! `cards/sorceries/mv_1/mind_peel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mind_peel() -> CardIndex {
    card_index("4f424d94-1544-4086-a57d-2add17332094")
}

/// Mind Peel (`4f424d94-1544-4086-a57d-2add17332094`) — {B} Sorcery (Partial):
/// "Target player discards a card."
#[test]
fn mind_peel_causes_target_player_to_discard_a_card() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp()])
        .hand(0, &[mind_peel()])
        .hand(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, mind_peel());
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    {
        engine
            .apply(
                player,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, P0, mind_peel()).is_some(),
        "base spell does not get free buyback"
    );

    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
}
