//! `cards/enchantments/auras/mv_1/dizzying_gaze.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn dizzying_gaze() -> CardIndex {
    card_index("e04f46d9-43e7-45ef-b59c-1810f30bec72")
}

/// Dizzying Gaze (`e04f46d9-43e7-45ef-b59c-1810f30bec72`) — {R} Enchantment — Aura (Partial):
/// "Enchant creature you control. {R}: Enchanted creature deals 1 damage to target creature with flying."
#[test]
fn dizzying_gaze_enchants_your_creature_and_deals_1_damage_to_target_creature_with_flying() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), index::GRIZZLY_BEARS])
        .battlefield(1, &[index::WIND_DRAKE])
        .hand(0, &[dizzying_gaze()])
        .start();
    assert!(walk_to_own_main(&mut engine, P0));

    let host = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let flyer = on_battlefield(&engine, P1, index::WIND_DRAKE).unwrap();
    cast_from_hand(&mut engine, P0, dizzying_gaze());
    aim_at(&mut engine, P0, host);
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, P0, dizzying_gaze()).unwrap();
    kit::activate_object(&mut engine, P0, aura, 1);
    aim_at(&mut engine, P0, flyer);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(kit::damage_on(&engine, flyer), 1);
}
