//! `cards/sorceries/mv_3/dirge_of_dread.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;

fn dirge_of_dread() -> CardIndex {
    card_index("be7b16ef-32aa-40d5-b287-c5e79d52d6b9")
}

/// Cast normally, Dirge of Dread gives ALL creatures across both players fear
/// until end of turn (restricting blockers to black or artifact creatures), and
/// wears off at cleanup.
#[test]
fn dirge_of_dread_cast_gives_all_creatures_fear_until_eot() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves(), festering_goblin(), ornithopter()])
        .hand(0, &[dirge_of_dread()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("P0 Elf");
    let opp_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("P1 Elf");
    let opp_goblin = on_battlefield(&engine, p1, festering_goblin()).expect("P1 Goblin");
    let opp_thopter = on_battlefield(&engine, p1, ornithopter()).expect("P1 Ornithopter");

    let lands = all_on_battlefield(&engine, p0, swamp());
    tap_mana_where(&mut engine, p0, |id| lands.contains(&id));
    cast_with_floating(&mut engine, p0, dirge_of_dread());
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, my_elf).contains(KeywordSet::FEAR));
    assert!(keywords(&engine, opp_elf).contains(KeywordSet::FEAR));

    let blockers = attack_and_collect_blocks(&mut engine, my_elf, p1);
    assert!(
        !may_block(&blockers, opp_elf, my_elf),
        "green creature cannot block fear"
    );
    assert!(
        may_block(&blockers, opp_goblin, my_elf),
        "black creature may block fear"
    );
    assert!(
        may_block(&blockers, opp_thopter, my_elf),
        "artifact creature may block fear"
    );
    block(&mut engine, &[]);
    through_combat(&mut engine);

    reach_their_main_phase(&mut engine, p1);
    assert!(!keywords(&engine, my_elf).contains(KeywordSet::FEAR));
    assert!(!keywords(&engine, opp_elf).contains(KeywordSet::FEAR));
}

/// Cycling Dirge of Dread for `{1}{B}` optionally grants fear to a single
/// target creature until end of turn (leaving others without fear), and may be
/// declined while still drawing.
#[test]
fn dirge_of_dread_cycling_grants_fear_to_one_target_or_declines() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                llanowar_elves(),
                serra_angel(),
            ],
        )
        .hand(0, &[dirge_of_dread(), dirge_of_dread()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Elf");
    let angel = on_battlefield(&engine, p0, serra_angel()).expect("Angel");
    let lands = all_on_battlefield(&engine, p0, swamp());
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // 1. Cycle and accept targeting Elf -> only Elf gains fear, before draw.
    tap_mana_where(&mut engine, p0, |id| id == lands[0] || id == lands[1]);
    activate(&mut engine, p0, dirge_of_dread(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[elf], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    yes_no(&mut engine, true);
    assert!(keywords(&engine, elf).contains(KeywordSet::FEAR));
    assert!(!keywords(&engine, angel).contains(KeywordSet::FEAR));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1
    );
    pass_until(&mut engine, |e| at_rest(e, p0));

    // 2. Cycle and decline targeting Angel -> Angel stays without fear, draws 1.
    tap_mana_where(&mut engine, p0, |id| id == lands[2] || id == lands[3]);
    activate(&mut engine, p0, dirge_of_dread(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[angel], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    yes_no(&mut engine, false);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(!keywords(&engine, angel).contains(KeywordSet::FEAR));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );

    reach_their_main_phase(&mut engine, p1);
    assert!(!keywords(&engine, elf).contains(KeywordSet::FEAR));
}
