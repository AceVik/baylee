//! `cards/enchantments/mv_6/recycle.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn recycle() -> CardIndex {
    card_index("8cef3ce7-8fbc-4e68-93f9-d6adfc2c2cbf")
}

/// `Recycle` ({4}{G}{G}, Enchantment, `Coverage::Partial`):
/// draws a card whenever you play a land or cast a spell.
#[test]
fn recycle_draws_a_card_when_you_play_a_land_or_cast_a_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), recycle()])
        .hand(0, &[forest(), GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let land_id = in_hand(&engine, p0, forest()).unwrap();
    engine
        .apply(p0, PlayerAction::PlayLand { card: land_id })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Playing a land spent 1 card and drew 1 card from Recycle's trigger.
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );

    // Casting a spell also triggers Recycle to draw a card.
    cast_from_hand(&mut engine, p0, GRIZZLY_BEARS);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, GRIZZLY_BEARS).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
}
