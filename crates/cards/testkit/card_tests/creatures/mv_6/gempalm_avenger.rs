//! `cards/creatures/mv_6/gempalm_avenger.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;

fn gempalm_avenger() -> CardIndex {
    card_index("cb9e6860-591f-4758-aaa3-14efa0a775f5")
}

fn elite_vanguard() -> CardIndex {
    card_index("b3b9a87d-cb95-435c-90b6-037406cab32e")
}

fn firdoch_core() -> CardIndex {
    card_index("126c75c0-414b-4e55-8747-2b5566ce716f")
}

/// Cycling Gempalm Avenger for `{2}{W}` pumps Soldier creatures on both sides
/// by +1/+1 and grants first strike until end of turn (resolving before the
/// cycling draw), skips non-Soldiers and noncreature Kindred permanents, and
/// expires at cleanup.
#[test]
fn gempalm_avenger_cycling_buffs_soldier_creatures_only_before_draw() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                elite_vanguard(),
                llanowar_elves(),
                firdoch_core(),
            ],
        )
        .battlefield(1, &[elite_vanguard()])
        .hand(0, &[gempalm_avenger()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sol0 = on_battlefield(&engine, p0, elite_vanguard()).expect("P0 Soldier");
    let elf0 = on_battlefield(&engine, p0, llanowar_elves()).expect("P0 Elf");
    let core = on_battlefield(&engine, p0, firdoch_core()).expect("noncreature Changeling");
    let sol1 = on_battlefield(&engine, p1, elite_vanguard()).expect("P1 Soldier");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, gempalm_avenger(), 0);
    pass_until(&mut engine, |e| {
        e.state().zones.list(ZoneLocation::Stack).len() == 1
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "CycledThis resolved before cycling draw"
    );
    assert_eq!(pt(&engine, sol0), (3, 2));
    assert!(keywords(&engine, sol0).contains(KeywordSet::FIRST_STRIKE));
    assert_eq!(pt(&engine, sol1), (3, 2));
    assert!(keywords(&engine, sol1).contains(KeywordSet::FIRST_STRIKE));
    assert_eq!(pt(&engine, elf0), (1, 1));
    assert!(!keywords(&engine, elf0).contains(KeywordSet::FIRST_STRIKE));
    assert!(!keywords(&engine, core).contains(KeywordSet::FIRST_STRIKE));

    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, sol0), (2, 1));
    assert!(!keywords(&engine, sol0).contains(KeywordSet::FIRST_STRIKE));
    assert_eq!(pt(&engine, sol1), (2, 1));
    assert!(!keywords(&engine, sol1).contains(KeywordSet::FIRST_STRIKE));
}
