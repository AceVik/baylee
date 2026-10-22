//! `cards/creatures/mv_1/jackal_pup.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn jackal_pup() -> CardIndex {
    card_index("993e41e6-0643-4a1f-8ada-b7a4aae486d9")
}

/// Jackal Pup (`993e41e6-0643-4a1f-8ada-b7a4aae486d9`) — {R} Creature — Jackal (Partial):
/// "Whenever this creature is dealt damage, it deals that much damage to you."
#[test]
fn jackal_pup_deals_damage_to_its_controller_when_dealt_damage() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[jackal_pup(), index::PRODIGAL_SORCERER])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let pup = on_battlefield(&engine, P0, jackal_pup()).unwrap();
    activate(&mut engine, P0, index::PRODIGAL_SORCERER, 0);
    aim_at(&mut engine, P0, pup);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, jackal_pup()).is_none());
    assert_eq!(engine.state().players[0].life, 19);
}
