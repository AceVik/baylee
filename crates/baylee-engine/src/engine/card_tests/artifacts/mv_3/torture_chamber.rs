//! `cards/artifacts/mv_3/torture_chamber.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

fn torture_chamber() -> CardIndex {
    card_index("12436f2f-eaed-4b46-8570-81316ce15962")
}

/// `Torture Chamber` ({3}, Artifact, `Coverage::Partial`):
/// `At the beginning of your upkeep, put a pain counter on this artifact.`
/// `At the beginning of your end step, this artifact deals damage to you equal
/// to the number of pain counters on it.`
/// `{1}, {T}, Remove all pain counters from this artifact: It deals damage to
/// target creature equal to the number of pain counters removed this way.`
#[test]
fn torture_chamber_accumulates_pain_counters_and_taps_to_damage_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), torture_chamber()])
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);

    // Turn 1 upkeep puts 1 pain counter on Torture Chamber.
    reach_main_phase(&mut engine, p0);
    let chamber = on_battlefield(&engine, p0, torture_chamber()).unwrap();
    assert_eq!(
        engine
            .state()
            .object(chamber)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::PAIN),
        1
    );

    // Activate {1}, {T} targeting Hill Giant -> deals 1 damage and removes all pain counters.
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, torture_chamber(), 2);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(giant).unwrap().damage, 1);
    assert_eq!(
        engine
            .state()
            .object(chamber)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::PAIN),
        0
    );
}
