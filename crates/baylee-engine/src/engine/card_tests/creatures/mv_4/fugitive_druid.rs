//! `cards/creatures/mv_4/fugitive_druid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn fugitive_druid() -> CardIndex {
    card_index("e05f3236-dc82-4353-87ab-d26ae7caa0e7")
}

/// Fugitive Druid (`e05f3236-dc82-4353-87ab-d26ae7caa0e7`) — {3}{G} Creature — Human Druid (Partial):
/// "Whenever this creature becomes the target of an Aura spell, you draw a card."
#[test]
fn fugitive_druid_does_not_offer_an_unrestricted_target_draw() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[fugitive_druid(), plains()])
        .hand(0, &[index::HOLY_STRENGTH])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let druid = on_battlefield(&engine, P0, fugitive_druid()).unwrap();
    cast_from_hand(&mut engine, P0, index::HOLY_STRENGTH);
    aim_at(&mut engine, P0, druid);
    let hand_after_cast = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_after_cast
    );
}
