//! Blinkmoth Urn — `{5}` Artifact (MRD).
//! Oracle: "At the beginning of each player's first main phase, if this artifact is untapped, that player adds {C} for each artifact they control."

use super::*;

fn blinkmoth_urn() -> CardIndex {
    card_index("08497e8f-3521-48e4-a1e7-c192b485d891")
}

#[test]
fn blinkmoth_urn_adds_colorless_mana_per_artifact_at_first_main_when_untapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let urn = blinkmoth_urn();
    let ring = sol_ring();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[urn, ring])
        .battlefield(1, &[ring])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    pass_until(&mut engine, stack_is_empty);

    // p0 controls Blinkmoth Urn + Sol Ring (2 artifacts) -> adds {C}{C}.
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        2
    );

    // Advance to p1's first main phase -> p1 controls 1 Sol Ring -> adds {C}.
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );
}
