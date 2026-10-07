//! `cards/enchantments/auras/mv_4/steal_artifact.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Steal Artifact: "Enchant artifact" / "You control enchanted artifact."
/// Same shape as Control Magic, over an artifact instead of a creature.
#[test]
fn steal_artifact_takes_the_artifact_and_gives_it_back_when_it_leaves() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let steal_artifact = steal_artifact();
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[sol_ring()])
        .hand(0, &[steal_artifact])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ring = on_battlefield(&engine, p1, sol_ring()).expect("their Sol Ring");

    cast_from_hand(&mut engine, p0, steal_artifact);
    aim_at(&mut engine, p0, ring);
    pass_until(&mut engine, stack_is_empty);
    let controller = |e: &Engine<RegistryLookup>| e.state().object(ring).unwrap().controller;
    assert_eq!(controller(&engine), p0, "you control enchanted artifact");

    let aura = on_battlefield(&engine, p0, steal_artifact).expect("Steal Artifact is attached");
    kill(&mut engine, aura);
    assert_eq!(
        controller(&engine),
        p1,
        "with the Aura gone, the artifact goes home"
    );
}
