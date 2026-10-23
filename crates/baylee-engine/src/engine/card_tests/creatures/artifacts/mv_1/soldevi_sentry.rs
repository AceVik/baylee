//! `cards/creatures/artifacts/mv_1/soldevi_sentry.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn soldevi_sentry() -> CardIndex {
    card_index("d2a61132-d881-4c80-92f1-a151d835832a")
}

/// Soldevi Sentry ({1} 1/1 Artifact Creature — Soldier):
/// - "{1}: Choose target opponent. Regenerate this creature. When it regenerates this way, that player may draw a card."
#[test]
fn soldevi_sentry_regenerates_itself_and_offers_target_opponent_a_draw() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[soldevi_sentry(), mountain(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sentry = on_battlefield(&engine, p0, soldevi_sentry()).unwrap();
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
    let p1_hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    tap_all_mana_but(&mut engine, p0, Some(soldevi_sentry()));
    activate(&mut engine, p0, soldevi_sentry(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p1),
    );
    engine.apply(p1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        p1_hand_before + 1
    );

    // Lethal damage from Lightning Bolt uses the regeneration shield (tapping Soldevi Sentry instead of destroying it).
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
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

    assert!(on_battlefield(&engine, p0, soldevi_sentry()).is_some());
    assert!(is_tapped(&engine, sentry));
}
