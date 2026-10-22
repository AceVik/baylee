//! `cards/creatures/mv_3/maddening_imp.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn maddening_imp() -> CardIndex {
    card_index("97dea59e-2090-4653-9ed9-86820a20605f")
}

/// Maddening Imp (`97dea59e-2090-4653-9ed9-86820a20605f`) — {2}{B} Creature — Imp:
/// "{T}: Non-Wall creatures the active player controls attack this turn if able. At the beginning of the next end step, destroy each of those creatures that didn't attack this turn. Activate only during an opponent's turn and only before combat."
#[test]
fn maddening_imp_forces_active_opponents_non_wall_creatures_to_attack() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[maddening_imp()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    // Pass priority on P1's first main phase to P0 so P0 can activate Maddening Imp before combat.
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    activate(&mut engine, P0, maddening_imp(), 0);
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    assert!(
        engine
            .apply(P1, PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err()
    );
}
