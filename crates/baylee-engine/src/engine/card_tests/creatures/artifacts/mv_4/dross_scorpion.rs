//! Dross Scorpion — `{4}` 3/1 Artifact Creature — Scorpion (MRD).
//! Oracle: "Whenever this creature or another artifact creature dies, you may untap target artifact."

use super::*;

fn dross_scorpion() -> CardIndex {
    card_index("132786bb-157c-4e2c-8f82-13df6a667448")
}

#[test]
fn dross_scorpion_untaps_target_artifact_when_it_or_another_artifact_creature_dies() {
    let p0 = PlayerId::new(0);
    let scorpion = dross_scorpion();
    let ring = sol_ring();
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[scorpion, ring, mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let scorpion_id = on_battlefield(&engine, p0, scorpion).unwrap();
    let ring_id = on_battlefield(&engine, p0, ring).unwrap();
    let mtn = on_battlefield(&engine, p0, mountain()).unwrap();

    // Tap Sol Ring for mana so it is tapped.
    tap_mana_where(&mut engine, p0, |id| id == ring_id || id == mtn);
    assert!(is_tapped(&engine, ring_id));

    // Kill Dross Scorpion with Lightning Bolt; its dies trigger untaps Sol Ring.
    cast_with_floating(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, scorpion_id);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at(&mut engine, p0, ring_id);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(!is_tapped(&engine, ring_id));
}
