//! `cards/instants/mv_2/imps_taunt.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn imps_taunt() -> CardIndex {
    card_index("7ebec169-d941-460e-9e3c-ba54b5066b05")
}

/// Imps' Taunt (`7ebec169-d941-460e-9e3c-ba54b5066b05`) — {1}{B} Instant (Partial):
/// "Buyback {3}. Target creature attacks this turn if able."
#[test]
fn imps_taunt_forces_target_creature_to_attack_this_turn() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), index::GRIZZLY_BEARS])
        .hand(0, &[imps_taunt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, imps_taunt());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    assert!(
        engine
            .apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err()
    );
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(P1))],
            },
        )
        .unwrap();
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert!(in_graveyard(&engine, P0, imps_taunt()).is_some());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
}
