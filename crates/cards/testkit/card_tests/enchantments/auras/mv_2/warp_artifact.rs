//! `cards/enchantments/auras/mv_2/warp_artifact.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Warp Artifact: "Enchant artifact" / "At the beginning of the upkeep of
/// enchanted artifact's controller, this Aura deals 1 damage to that
/// player."
#[test]
fn warp_artifact_deals_upkeep_damage_to_the_enchanted_artifacts_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let warp_artifact = warp_artifact();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp()])
        .battlefield(1, &[sol_ring()])
        .hand(0, &[warp_artifact])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ring = on_battlefield(&engine, p1, sol_ring()).expect("their Sol Ring");

    cast_from_hand(&mut engine, p0, warp_artifact);
    aim_at(&mut engine, p0, ring);
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().players[1].life,
        19,
        "1 damage at its controller's upkeep"
    );
    assert_eq!(engine.state().players[0].life, 20);

    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "not at Warp Artifact's own controller's upkeep, only the enchanted artifact's"
    );
    assert_eq!(engine.state().players[1].life, 19);
}
