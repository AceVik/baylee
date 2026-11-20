//! `cards/creatures/mv_2/tribal_forcemage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::subtypes;

fn tribal_forcemage() -> CardIndex {
    card_index("245934d9-c7b7-4a97-9c56-7760d934ef27")
}

/// Turning Tribal Forcemage face up via morph `{1}{G}` pumps all creatures of
/// the chosen type (including itself if matching and opponent's creatures) by
/// +2/+2 and grants trample until end of turn.
#[test]
fn tribal_forcemage_morphs_face_up_and_pumps_chosen_type_with_trample() {
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
                llanowar_elves(),
                festering_goblin(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[tribal_forcemage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf0 = on_battlefield(&engine, p0, llanowar_elves()).expect("P0 Elf");
    let goblin0 = on_battlefield(&engine, p0, festering_goblin()).expect("P0 Goblin");
    let elf1 = on_battlefield(&engine, p1, llanowar_elves()).expect("P1 Elf");

    // Cast face down for {3} (leaving 2 Forests for Morph {1}{G}).
    let first3: Vec<ObjectId> = lands_of(&engine, p0).into_iter().take(3).collect();
    tap_mana_where(&mut engine, p0, |id| first3.contains(&id));
    cast_with_floating(&mut engine, p0, tribal_forcemage());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected morph choice: {:?}", engine.pending());
    };
    let morph = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Disguise))
        .expect("morph cast mode offered");
    engine
        .apply(p0, PlayerAction::ChooseMode(morph))
        .expect("cast face down");
    pass_until(&mut engine, stack_is_empty);

    let mage = on_battlefield(&engine, p0, tribal_forcemage()).expect("face-down permanent");
    assert_eq!(pt(&engine, mage), (2, 2));

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: mage,
                ability_index: baylee_engine::choice::TURN_FACE_UP,
            },
        )
        .expect("turn Tribal Forcemage face up for {1}{G}");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::ELF))
        .expect("choose Elf");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(pt(&engine, mage), (3, 3), "1/1 Elf Wizard pumped +2/+2");
    assert!(keywords(&engine, mage).contains(KeywordSet::TRAMPLE));
    assert_eq!(pt(&engine, elf0), (3, 3));
    assert!(keywords(&engine, elf0).contains(KeywordSet::TRAMPLE));
    assert_eq!(pt(&engine, elf1), (3, 3));
    assert!(keywords(&engine, elf1).contains(KeywordSet::TRAMPLE));
    assert_eq!(pt(&engine, goblin0), (1, 1));
    assert!(!keywords(&engine, goblin0).contains(KeywordSet::TRAMPLE));

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, mage), (1, 1));
    assert!(!keywords(&engine, mage).contains(KeywordSet::TRAMPLE));
    assert_eq!(pt(&engine, elf0), (1, 1));
    assert!(!keywords(&engine, elf0).contains(KeywordSet::TRAMPLE));
}
