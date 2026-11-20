//! `cards/enchantments/mv_2/shared_triumph.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::subtypes;

fn shared_triumph() -> CardIndex {
    card_index("64d022ab-5222-4a7a-ab8a-89749560f666")
}

fn woodland_changeling() -> CardIndex {
    card_index("338f0ab6-f744-4e5a-a358-05e71b93758d")
}

/// Shared Triumph gives creatures of the chosen type +1/+1 across all seats,
/// applies continuously to creatures that enter later, and leaves other types
/// unchanged.
#[test]
fn shared_triumph_pumps_chosen_creature_type_across_all_players() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                forest(),
                llanowar_elves(),
                festering_goblin(),
            ],
        )
        .battlefield(1, &[llanowar_elves(), woodland_changeling()])
        .hand(0, &[shared_triumph(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf0 = on_battlefield(&engine, p0, llanowar_elves()).expect("P0 Elf seated");
    let goblin0 = on_battlefield(&engine, p0, festering_goblin()).expect("P0 Goblin seated");
    let elf1 = on_battlefield(&engine, p1, llanowar_elves()).expect("P1 Elf seated");
    let changeling1 =
        on_battlefield(&engine, p1, woodland_changeling()).expect("P1 Changeling seated");

    cast_from_hand(&mut engine, p0, shared_triumph());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::ELF))
        .expect("choose Elf for Shared Triumph");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elf0), (2, 2));
    assert_eq!(pt(&engine, elf1), (2, 2));
    assert_eq!(
        pt(&engine, changeling1),
        (3, 3),
        "Changeling creature has the chosen type and gets +1/+1"
    );
    assert_eq!(pt(&engine, goblin0), (1, 1), "non-Elf Goblin is untouched");

    // Static continuous effect also buffs an Elf that enters afterward.
    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2);
    for id in elves {
        assert_eq!(pt(&engine, id), (2, 2));
    }
}
