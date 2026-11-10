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

/// "Attached to it": of two Masks, one equips the Punisher and one the
/// Ogre beside it. Only the first counts, so the Punisher is 4/4, not 6/6.
#[test]
fn loxodon_punisher_ignores_equipment_on_another_creature() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                loxodon_punisher(),
                gray_ogre(),
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
    let ogre = on_battlefield(&e, p0, gray_ogre()).unwrap();
    let masks = all_on_battlefield(&e, p0, mask_of_memory());
    for (equipment, host) in masks.into_iter().zip([punisher, ogre]) {
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
                objects: vec![host],
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().object(equipment).unwrap().attached_to, Some(host));
    }
    assert_eq!(pt(&e, punisher), (4, 4));
    assert_eq!(pt(&e, ogre), (2, 2));
}
