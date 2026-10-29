//! Frogmite — `{4}` 2/2 Artifact Creature — Frog (MRD).
//! Oracle: "Affinity for artifacts (This spell costs {1} less to cast for each artifact you control.)"

use super::*;

fn frogmite() -> CardIndex {
    card_index("a927fd33-f183-48d9-b582-55d379d70e13")
}

#[test]
fn frogmite_costs_zero_with_four_artifacts_you_control() {
    let p0 = PlayerId::new(0);
    let frog = frogmite();
    let ring = sol_ring();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ring, ring, ring, ring])
        .hand(0, &[frog])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Cast Frogmite for {0} without tapping any mana abilities.
    let frog_card = in_hand(&engine, p0, frog).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: frog_card })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, frog).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}
