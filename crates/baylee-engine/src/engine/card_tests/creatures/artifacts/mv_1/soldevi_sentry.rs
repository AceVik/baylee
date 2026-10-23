//! `cards/creatures/artifacts/mv_1/soldevi_sentry.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn soldevi_sentry() -> CardIndex {
    card_index("d2a61132-d881-4c80-92f1-a151d835832a")
}

/// Soldevi Sentry ({1} 1/1 Artifact Creature — Soldier, Partial):
/// "{1}: Choose target opponent. Regenerate this creature. When it
/// regenerates this way, that player may draw a card."
/// The drawback (the opponent's card) cannot be said, and regeneration without
/// it would play stronger than printed, so the card offers no ability. A pinned
/// limitation: this test moves when a regeneration shield can trigger.
#[test]
fn soldevi_sentry_offers_no_regeneration_without_its_drawback() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[soldevi_sentry(), mountain(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sentry = on_battlefield(&engine, p0, soldevi_sentry()).unwrap();
    let p1_hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    tap_all_mana_but(&mut engine, p0, Some(soldevi_sentry()));
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == sentry),
        "the {{1}} ability is not offered"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: sentry,
                    ability_index: 0,
                },
            )
            .is_err()
    );

    // No shield: Lightning Bolt destroys the Sentry, and nobody drew.
    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![sentry],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, soldevi_sentry()).is_none());
    assert!(in_graveyard(&engine, p0, soldevi_sentry()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        p1_hand_before
    );
}
