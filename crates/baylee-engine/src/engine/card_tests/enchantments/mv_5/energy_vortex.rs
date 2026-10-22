//! `cards/enchantments/mv_5/energy_vortex.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::ENERGY_VORTEX;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Energy Vortex (bcb2f489-9d58-424d-96c1-a124a8d25a14) — {3}{U}{U} Enchantment:
/// "As this enchantment enters, choose an opponent. At the beginning of your
/// upkeep, remove all vortex counters from this enchantment. At the beginning
/// of the chosen player's upkeep, this enchantment deals 3 damage to that
/// player unless they pay {1} for each vortex counter on this enchantment.
/// {X}: Put X vortex counters on this enchantment. Activate only during your
/// upkeep."
#[test]
fn energy_vortex_adds_vortex_counters_in_upkeep_and_deals_damage_to_chosen_opponent() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[ENERGY_VORTEX])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, ENERGY_VORTEX);
    for _ in 0..10 {
        if stack_is_empty(&engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            break;
        }
        match engine.pending().clone() {
            Pending::ChoosePlayer { player, .. } => {
                engine
                    .apply(player, PlayerAction::ChoosePlayer(P1))
                    .unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }

    let vortex = on_battlefield(&engine, P0, ENERGY_VORTEX).unwrap();

    // Advance to P0's next upkeep (turn 3), answering P1's turn-2 upkeep tax prompt along the way.
    for _ in 0..80 {
        if engine.state().turn.active == P0
            && engine.state().turn.number == 3
            && engine.state().turn.step == Step::Upkeep
            && stack_is_empty(&engine)
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == P0)
        {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(true)).unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, ENERGY_VORTEX, 2);
    engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(vortex)
            .unwrap()
            .counters
            .get(counters::VORTEX),
        2
    );

    // Advance to P1's upkeep (turn 4): P1 declines to pay {2}, so Energy Vortex deals 3 damage.
    for _ in 0..80 {
        if engine.state().turn.active == P1 && engine.state().turn.phase == Phase::FirstMain {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(false)).unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    assert_eq!(engine.state().players[1].life, 17);
}
