//! `cards/enchantments/mv_2/cover_of_darkness.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;
use baylee_core::generated::subtypes;

fn cover_of_darkness() -> CardIndex {
    card_index("d78654fc-7d90-40ce-9c36-a211424d1a88")
}

/// Cover of Darkness grants fear to every creature of the chosen type across
/// both seats (limiting blockers to black or artifact creatures), while leaving
/// creatures of other types unaffected.
#[test]
fn cover_of_darkness_grants_fear_to_chosen_type() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), llanowar_elves(), festering_goblin()])
        .battlefield(
            1,
            &[
                llanowar_elves(),
                serra_angel(),
                festering_goblin(),
                ornithopter(),
            ],
        )
        .hand(0, &[cover_of_darkness()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("P0 Elf seated");
    let my_goblin = on_battlefield(&engine, p0, festering_goblin()).expect("P0 Goblin seated");
    let opp_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("P1 Elf seated");
    let opp_angel = on_battlefield(&engine, p1, serra_angel()).expect("P1 Angel seated");
    let opp_goblin = on_battlefield(&engine, p1, festering_goblin()).expect("P1 Goblin seated");
    let opp_thopter = on_battlefield(&engine, p1, ornithopter()).expect("P1 Ornithopter seated");

    let swamps = all_on_battlefield(&engine, p0, swamp());
    tap_mana_where(&mut engine, p0, |id| swamps.contains(&id));
    cast_with_floating(&mut engine, p0, cover_of_darkness());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::ELF))
        .expect("choose Elf for Cover of Darkness");
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, my_elf).contains(KeywordSet::FEAR));
    assert!(
        keywords(&engine, opp_elf).contains(KeywordSet::FEAR),
        "opponent's Elf also gains fear"
    );
    assert!(
        !keywords(&engine, my_goblin).contains(KeywordSet::FEAR),
        "Goblin does not gain fear when Elf was chosen"
    );

    let blockers = attack_and_collect_blocks(&mut engine, my_elf, p1);
    assert!(
        !may_block(&blockers, opp_angel, my_elf),
        "white non-artifact Serra Angel cannot block creature with fear"
    );
    assert!(
        !may_block(&blockers, opp_elf, my_elf),
        "green non-artifact Llanowar Elves cannot block creature with fear"
    );
    assert!(
        may_block(&blockers, opp_goblin, my_elf),
        "black creature may block creature with fear"
    );
    assert!(
        may_block(&blockers, opp_thopter, my_elf),
        "artifact creature may block creature with fear"
    );
}
