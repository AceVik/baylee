//! `cards/creatures/mv_2/acidic_sliver.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn acidic_sliver() -> CardIndex {
    card_index("a149eeb1-007c-435d-9d55-34af205090e0")
}

/// Acidic Sliver (`a149eeb1-007c-435d-9d55-34af205090e0`) — {B}{R} 2/2 Creature — Sliver (Implemented):
/// "All Slivers have '{2}, Sacrifice this permanent: This permanent deals 2 damage to any target.'"
#[test]
fn acidic_sliver_grants_sacrifice_two_damage_ability() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), mountain(), forest(), forest()])
        .hand(0, &[acidic_sliver()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, acidic_sliver());
    pass_until(&mut engine, stack_is_empty);

    let sliver = on_battlefield(&mut engine, P0, acidic_sliver()).unwrap();
    tap_all_mana(&mut engine, P0);
    let idx = kit::offered(&engine, sliver)[0];
    kit::activate_object(&mut engine, P0, sliver, idx);
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

    assert!(in_graveyard(&engine, P0, acidic_sliver()).is_some());
    assert_eq!(engine.state().players[1].life, 18);
}
