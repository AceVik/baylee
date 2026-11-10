//! Task Mage Assembly — {2}{R} — Enchantment
//! Oracle: When there are no creatures on the battlefield, sacrifice this enchantment.
//! Oracle: {2}: This enchantment deals 1 damage to target creature. Any player may activate this ability but only as a sorcery.

use super::*;

#[test]
fn task_mage_assembly_deals_damage_and_sacrifices_itself_when_no_creatures_remain() {
    let task_mage_assembly = card_index("dfbbfbe1-d6c1-4b5b-8d09-949d70b9ff42");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[task_mage_assembly])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, task_mage_assembly);
    pass_until(&mut engine, stack_is_empty);

    let assembly_id =
        on_battlefield(&engine, p0, task_mage_assembly).expect("Task Mage Assembly on board");
    let elf_id = on_battlefield(&engine, p1, llanowar_elves()).expect("Llanowar Elves on board");

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: assembly_id,
                ability_index: 1,
            },
        )
        .expect("activate Task Mage Assembly damage ability");
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![elf_id],
                },
            )
            .expect("target Llanowar Elves");
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, llanowar_elves()).is_some());
    assert!(on_battlefield(&engine, p0, task_mage_assembly).is_none());
    assert!(in_graveyard(&engine, p0, task_mage_assembly).is_some());
}
