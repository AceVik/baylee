//! Auriok Steelshaper — `{1}{W}` 1/1 Creature — Human Soldier (MRD).
//! Oracle: "Equip costs you pay cost {1} less. As long as this creature is equipped, each creature you control that's a Soldier or a Knight gets +1/+1."

use super::*;

fn auriok_steelshaper() -> CardIndex {
    card_index("e3ee8b0e-9bdc-4b9a-b4f9-59189dd6a80d")
}

#[test]
fn auriok_steelshaper_reduces_equip_costs_by_one_and_pumps_soldiers_and_knights_when_equipped() {
    let p0 = PlayerId::new(0);
    let shaper = auriok_steelshaper();
    let sledge = card_index("e1ee79de-e4c3-48f9-87a7-18f72e2f6116");
    let elf = llanowar_elves();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[shaper, shaper, elf, sledge, plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let shapers = all_on_battlefield(&engine, p0, shaper);
    let elf_id = on_battlefield(&engine, p0, elf).unwrap();
    assert_eq!(pt(&engine, shapers[0]), (1, 1));
    assert_eq!(pt(&engine, shapers[1]), (1, 1));

    // Dead-Iron Sledge normally costs {2} to equip; 2 Auriok Steelshapers reduce it to {0},
    // or 1 Plains pays {1} with 1 Steelshaper.
    tap_all_mana_but(&mut engine, p0, Some(elf));
    activate(&mut engine, p0, sledge, 0);
    aim_at(&mut engine, p0, shapers[0]);
    pass_until(&mut engine, stack_is_empty);

    // Equipped Auriok Steelshaper pumps all Soldiers/Knights you control (+1/+1), leaving the Elf at 1/1.
    assert_eq!(pt(&engine, shapers[0]), (2, 2));
    assert_eq!(pt(&engine, shapers[1]), (2, 2));
    assert_eq!(pt(&engine, elf_id), (1, 1));
}

#[test]
fn auriok_steelshaper_does_not_reduce_an_equipments_other_abilities() {
    let p0 = PlayerId::new(0);
    let shaper = auriok_steelshaper();
    let scimitar = card_index("aa5865f5-aafc-4e46-995b-f005aeefde91");
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[shaper, scimitar])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let scimitar = on_battlefield(&engine, p0, scimitar).unwrap();
    let cost = |mana| baylee_cards_dsl::Cost {
        mana: baylee_core::mana::ManaCost::parse(mana),
        parts: &[],
    };
    assert_eq!(
        engine
            .activation_price(p0, scimitar, 0, &cost("{2}"), None)
            .mana
            .generic_total(),
        1,
        "the actual equip ability is reduced"
    );
    assert_eq!(
        engine
            .activation_price(p0, scimitar, 1, &cost("{3}"), None)
            .mana
            .generic_total(),
        3,
        "the Scimitar's separate animation ability keeps its printed price"
    );
}
