//! Loxodon Punisher gets +2/+2 per attached Equipment.

use super::*;

fn loxodon_punisher() -> CardIndex {
    card_index("fc3869fd-1441-4d5e-9af5-f02857ed634a")
}

fn mask_of_memory() -> CardIndex {
    card_index("d6b2c998-a226-426c-a40d-6e6007041bfe")
}

#[test]
fn loxodon_punisher_gets_four_four_from_two_equipment() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                loxodon_punisher(),
                mask_of_memory(),
                mask_of_memory(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let punisher = on_battlefield(&e, p0, loxodon_punisher()).unwrap();
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
                objects: vec![punisher],
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
    }
    assert_eq!(pt(&e, punisher), (6, 6));
}
