//! `cards/instants/mv_2/tribal_unity.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;
use baylee_core::generated::subtypes;

fn tribal_unity() -> CardIndex {
    card_index("6b4ee935-4474-4757-9fab-978a2e1e72a0")
}

/// Tribal Unity pumps all creatures currently of the chosen type across both
/// players by +X/+X until end of turn, ignores other types and later entrants,
/// and wears off at cleanup.
#[test]
fn tribal_unity_pumps_chosen_type_by_x_until_eot() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                festering_goblin(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[tribal_unity(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf0 = on_battlefield(&engine, p0, llanowar_elves()).expect("P0 Elf");
    let goblin0 = on_battlefield(&engine, p0, festering_goblin()).expect("P0 Goblin");
    let elf1 = on_battlefield(&engine, p1, llanowar_elves()).expect("P1 Elf");

    // Tap 6 Forests for X = 3 ({3}{2}{G}), leaving 1 Forest to cast another Elf afterward.
    let tap6: Vec<ObjectId> = lands_of(&engine, p0).into_iter().take(6).collect();
    tap_mana_where(&mut engine, p0, |id| tap6.contains(&id));
    announce(&mut engine, p0, tribal_unity(), 0, 3, &[], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::ELF))
        .expect("choose Elf for Tribal Unity");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(pt(&engine, elf0), (4, 4));
    assert_eq!(pt(&engine, elf1), (4, 4));
    assert_eq!(pt(&engine, goblin0), (1, 1), "non-Elf is untouched");

    // CR 611.2c: a creature entering after resolution does not receive the pump.
    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    let late_elf = all_on_battlefield(&engine, p0, llanowar_elves())
        .into_iter()
        .find(|id| *id != elf0)
        .expect("late Elf entered");
    assert_eq!(pt(&engine, late_elf), (1, 1));

    // Reverts at cleanup.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, elf0), (1, 1));
    assert_eq!(pt(&engine, elf1), (1, 1));
}
