//! `cards/creatures/mv_6/karona_false_god.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;
use baylee_core::generated::subtypes;

fn karona_false_god() -> CardIndex {
    card_index("9ea1f10c-5ceb-4ac0-84c6-29046dea649a")
}

/// Karona, False God has haste (can attack the turn she enters), gives
/// creatures of the chosen type +3/+3 until end of turn whenever she attacks
/// across all players' boards, and changes control & untaps at the beginning
/// of each player's upkeep.
#[test]
fn karona_false_god_has_haste_pumps_on_attack_and_changes_control_each_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                island(),
                swamp(),
                mountain(),
                forest(),
                forest(),
                llanowar_elves(),
                festering_goblin(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[karona_false_god()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf0 = on_battlefield(&engine, p0, llanowar_elves()).expect("P0 Elf");
    let goblin0 = on_battlefield(&engine, p0, festering_goblin()).expect("P0 Goblin");
    let elf1 = on_battlefield(&engine, p1, llanowar_elves()).expect("P1 Elf");

    cast_from_hand(&mut engine, p0, karona_false_god());
    pass_until(&mut engine, stack_is_empty);
    let karona = on_battlefield(&engine, p0, karona_false_god()).expect("Karona entered");
    assert!(keywords(&engine, karona).contains(KeywordSet::HASTE));

    // Attacks immediately on the turn she entered; choose Elf on attack trigger.
    let attackers = attackers_offered(&mut engine);
    assert!(
        attackers.contains(&karona),
        "Haste lets Karona attack immediately"
    );
    e_declare_attack(&mut engine, karona, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::ELF))
        .expect("choose Elf on P0 attack trigger");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    block(&mut engine, &[]);
    through_combat(&mut engine);

    assert_eq!(pt(&engine, elf0), (4, 4), "P0 Elf gets +3/+3");
    assert_eq!(pt(&engine, elf1), (4, 4), "P1 Elf also gets +3/+3");
    assert_eq!(pt(&engine, goblin0), (1, 1), "non-Elf stays 1/1");
    assert_eq!(pt(&engine, karona), (5, 5), "Avatar stays 5/5");
    assert!(is_tapped(&engine, karona));
    assert_eq!(life(&engine, p1), 15, "5 combat damage dealt to P1");

    // P1's upkeep untaps Karona and hands control to P1; Elf buff wore off.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, elf0), (1, 1));
    assert_eq!(pt(&engine, elf1), (1, 1));
    assert_eq!(engine.state().object(karona).unwrap().controller, p1);
    assert!(!is_tapped(&engine, karona), "P1's upkeep untapped Karona");

    // P1 attacks P0 with Karona and chooses Goblin -> P0's Goblin gets +3/+3!
    let p1_attackers = attackers_offered(&mut engine);
    assert!(p1_attackers.contains(&karona));
    e_declare_attack(&mut engine, karona, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p1, PlayerAction::ChooseSubtype(subtypes::creature::GOBLIN))
        .expect("P1 chooses Goblin");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    block(&mut engine, &[]);
    through_combat(&mut engine);

    assert_eq!(pt(&engine, goblin0), (4, 4));
    assert_eq!(pt(&engine, elf0), (1, 1));
    assert_eq!(life(&engine, p0), 15, "5 combat damage dealt to P0");

    // Back to P0's turn: P0's upkeep untaps Karona and returns control to P0.
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(engine.state().object(karona).unwrap().controller, p0);
    assert!(!is_tapped(&engine, karona));
}

fn e_declare_attack(engine: &mut Engine<RegistryLookup>, attacker: ObjectId, defender: PlayerId) {
    let Pending::ChooseAttackers { player, .. } = engine.pending().clone() else {
        panic!("expected ChooseAttackers, got {:?}", engine.pending());
    };
    engine
        .apply(
            player,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(defender))],
            },
        )
        .expect("declare Karona attacking");
}
