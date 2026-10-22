//! `cards/instants/mv_2/illumination.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{ILLUMINATION, JAYEMDAE_TOME};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Illumination (eb28f6e5-c9ef-416e-b315-967d857e7600) — {W}{W} Instant:
/// "Counter target artifact or enchantment spell. Its controller gains life
/// equal to its mana value."
#[test]
fn illumination_counters_artifact_spell_and_gives_controller_life_equal_to_mana_value() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .battlefield(1, &[plains(), plains()])
        .hand(0, &[JAYEMDAE_TOME])
        .hand(1, &[ILLUMINATION])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, JAYEMDAE_TOME);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    let spell_id = engine.state().zones.list(ZoneLocation::Stack)[0];
    tap_all_mana(&mut engine, P1);
    cast_with_floating(&mut engine, P1, ILLUMINATION);
    aim_at(&mut engine, P1, spell_id);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, JAYEMDAE_TOME).is_none());
    // Jayemdae Tome has mana value 4 -> P0 gains 4 life (20 -> 24).
    assert_eq!(engine.state().players[0].life, 24);
}
