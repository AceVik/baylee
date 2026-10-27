//! `cards/creatures/mv_2/samite_pilgrim.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Samite Pilgrim — {1}{W} Creature — Human Cleric 1/1
/// "Domain — {T}: Prevent the next X damage that would be dealt to target creature
/// this turn, where X is the number of basic land types among lands you control."
#[test]
fn samite_pilgrim_prevents_damage_equal_to_basic_land_types_you_control() {
    let pilgrim = card_index("542c5eaf-8b03-4406-a794-30c56b61d637");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[pilgrim, grizzly_bears(), plains(), island(), mountain()],
        )
        .hand(0, &[index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let pilgrim_id = on_battlefield(&engine, p0, pilgrim).expect("Samite Pilgrim on board");
    let bears_id = on_battlefield(&engine, p0, grizzly_bears()).expect("Grizzly Bears on board");

    // 3 basic land types (Plains, Island, Mountain) -> prevent the next 3 damage.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: pilgrim_id,
                ability_index: 0,
            },
        )
        .unwrap();
    aim_at(&mut engine, p0, bears_id);
    pass_until(&mut engine, stack_is_empty);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, index::LIGHTNING_BOLT);
    aim_at(&mut engine, p0, bears_id);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());
    assert_eq!(engine.state().object(bears_id).unwrap().damage, 0);
}
