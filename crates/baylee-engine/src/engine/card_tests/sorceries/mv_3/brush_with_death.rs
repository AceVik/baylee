//! `cards/sorceries/mv_3/brush_with_death.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn brush_with_death() -> CardIndex {
    card_index("d87e580f-eca1-4f91-962b-82d684cba5e7")
}

/// Brush with Death (`d87e580f-eca1-4f91-962b-82d684cba5e7`) — {2}{B} Sorcery (Partial):
/// "Target opponent loses 2 life and you gain 2 life."
#[test]
fn brush_with_death_drains_target_opponent_for_two() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[brush_with_death()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, brush_with_death());
    assert!(engine.apply(P0, PlayerAction::ChoosePlayer(P0)).is_err());
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, P0, brush_with_death()).is_some(),
        "base spell does not get free buyback"
    );

    assert_eq!(engine.state().players[1].life, 18);
    assert_eq!(engine.state().players[0].life, 22);
}
