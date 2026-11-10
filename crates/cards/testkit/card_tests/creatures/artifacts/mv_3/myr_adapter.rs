//! Myr Adapter gets +1/+1 per attached Equipment.

use super::*;

fn myr_adapter() -> CardIndex {
    card_index("714f916a-10ac-4c54-b1c7-a97b7ad8b9e5")
}

fn mask_of_memory() -> CardIndex {
    card_index("d6b2c998-a226-426c-a40d-6e6007041bfe")
}

#[test]
fn myr_adapter_counts_each_equipment_attached_to_it() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                myr_adapter(),
                mask_of_memory(),
                mask_of_memory(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let adapter = on_battlefield(&e, p0, myr_adapter()).unwrap();
    assert_eq!(pt(&e, adapter), (1, 1));
    for equipment in all_on_battlefield(&e, p0, mask_of_memory()) {
        tap_all_mana(&mut e, p0);
        e.apply(
            p0,
            PlayerAction::ActivateAbility {
                source: equipment,
                ability_index: 0,
            },
        )
        .unwrap();
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![adapter],
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
    }
    assert_eq!(pt(&e, adapter), (3, 3));
}
