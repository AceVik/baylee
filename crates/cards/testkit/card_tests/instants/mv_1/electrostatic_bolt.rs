//! `cards/instants/mv_1/electrostatic_bolt.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{HILL_GIANT, JUGGERNAUT};

fn electrostatic_bolt() -> CardIndex {
    card_index("6fc26faa-94a8-433f-8863-126c2d2e73b0")
}

/// Electrostatic Bolt — {R} Instant
/// "Electrostatic Bolt deals 2 damage to target creature. If it's an artifact creature, Electrostatic Bolt deals 4 damage to it instead."
#[test]
fn electrostatic_bolt_deals_two_to_nonartifact_creature_and_four_to_artifact_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[electrostatic_bolt(), electrostatic_bolt()])
        .battlefield(0, &[mountain(), mountain()])
        .battlefield(1, &[HILL_GIANT, JUGGERNAUT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
    let juggernaut = on_battlefield(&engine, p1, JUGGERNAUT).unwrap();

    // First Electrostatic Bolt on Hill Giant (3/3 non-artifact creature) deals 2 damage.
    cast_from_hand(&mut engine, p0, electrostatic_bolt());
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(giant).unwrap().damage, 2);

    // Second Electrostatic Bolt on Juggernaut (5/3 artifact creature) deals 4 lethal damage.
    cast_from_hand(&mut engine, p0, electrostatic_bolt());
    aim_at(&mut engine, p0, juggernaut);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, JUGGERNAUT).is_none());
}
