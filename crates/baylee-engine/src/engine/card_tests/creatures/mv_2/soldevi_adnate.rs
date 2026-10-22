//! `cards/creatures/mv_2/soldevi_adnate.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn soldevi_adnate() -> CardIndex {
    card_index("e3174070-542f-429e-9149-b572f6614537")
}

fn sengir_vampire() -> CardIndex {
    baylee_core::generated::index::SENGIR_VAMPIRE
}

/// Soldevi Adnate ({1}{B} 1/2 Creature — Human Cleric):
/// - "{T}, Sacrifice a black or artifact creature: Add an amount of {B} equal to the sacrificed creature's mana value."
#[test]
fn soldevi_adnate_sacrifices_black_creature_to_add_black_mana_equal_to_mana_value() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[soldevi_adnate(), sengir_vampire()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let adnate = on_battlefield(&engine, p0, soldevi_adnate()).unwrap();
    let vampire = on_battlefield(&engine, p0, sengir_vampire()).unwrap();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: adnate,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![vampire],
            },
        )
        .unwrap();

    assert!(in_graveyard(&engine, p0, sengir_vampire()).is_some());
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        5
    );
}
