//! `cards/enchantments/mv_1/alms.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

fn alms() -> CardIndex {
    card_index("efd46a6e-a901-4224-8a29-d53a4dc7750c")
}

/// `Alms` ({W}, Enchantment, `Coverage::Partial`) activates for `{1}` and
/// exiling a card from your graveyard to prevent the next 1 damage to target
/// creature this turn.
#[test]
fn alms_exiles_graveyard_card_to_prevent_one_damage_to_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), mountain(), alms(), HILL_GIANT])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 1);
    let gy_card = engine.state().zones.list(ZoneLocation::Graveyard(p0))[0];
    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, alms(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![gy_card],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, p0, lightning_bolt());
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

    assert!(on_battlefield(&engine, p0, HILL_GIANT).is_some());
}
