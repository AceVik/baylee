//! `cards/instants/mv_1/anoint.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn anoint() -> CardIndex {
    card_index("fcb03c54-5b9a-428d-bbf6-ef8977770c8d")
}

/// Anoint (fcb03c54-5b9a-428d-bbf6-ef8977770c8d) — {W} Instant (Partial):
/// "Prevent the next 3 damage that would be dealt to target creature this turn."
#[test]
fn anoint_prevents_next_three_damage_to_target_creature() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), mountain(), index::GRIZZLY_BEARS])
        .hand(0, &[anoint(), index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, anoint());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, P0, index::LIGHTNING_BOLT);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some());
    assert_eq!(engine.state().object(bears).unwrap().damage, 0);
}
