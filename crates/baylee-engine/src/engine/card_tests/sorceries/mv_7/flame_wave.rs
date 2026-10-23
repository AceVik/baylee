//! `cards/sorceries/mv_7/flame_wave.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn flame_wave() -> CardIndex {
    card_index("cee8ec5f-e36d-4bca-9b19-cb33113835b1")
}

/// Flame Wave (`cee8ec5f-e36d-4bca-9b19-cb33113835b1`) — {3}{R}{R}{R}{R} Sorcery (Partial):
/// "This spell deals 4 damage to target player or planeswalker and 4 damage to each creature that player or that planeswalker's controller controls."
#[test]
fn flame_wave_deals_four_damage_to_target_player_and_opponent_creatures() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[flame_wave()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, flame_wave());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 16);
    assert!(in_graveyard(&engine, P1, index::HILL_GIANT).is_some());
}
