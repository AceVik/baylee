//! `cards/artifacts/mv_3/horn_of_greed.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn horn_of_greed() -> CardIndex {
    card_index("b8181d53-1954-4f46-8670-8696440208e8")
}

/// Horn of Greed (`b8181d53-1954-4f46-8670-8696440208e8`) — {3} Artifact (Implemented):
/// "Whenever a player plays a land, that player draws a card."
#[test]
fn horn_of_greed_draws_a_card_when_a_player_plays_a_land() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[horn_of_greed()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    play_land(&mut engine, P0, forest());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before,
        "playing a land (-1) and drawing from Horn of Greed (+1) leaves hand size unchanged"
    );
}
