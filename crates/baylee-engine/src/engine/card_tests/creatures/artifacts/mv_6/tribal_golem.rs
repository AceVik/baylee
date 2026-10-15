//! `cards/creatures/artifacts/mv_6/tribal_golem.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn tribal_golem() -> CardIndex {
    card_index("9027d519-2e89-404f-b357-22799aee1874")
}

fn slippery_bogle() -> CardIndex {
    card_index("f64878c1-1fba-44f7-a24f-d24bef2e03ae")
}

fn mogg_fanatic() -> CardIndex {
    card_index("7efd5d62-6da2-428a-98e3-5b842f668656")
}

fn carrion_feeder() -> CardIndex {
    card_index("a1cc5e37-b09a-4b7f-afd5-77c1c35aa425")
}

/// Tribal Golem — each creature type you control switches on one thing:
/// Beast trample, Goblin haste, Soldier first strike, Wizard flying. Each is
/// checked alone: only its own keyword appears.
#[test]
fn tribal_golem_reads_each_creature_type_it_is_printed_with() {
    let p0 = PlayerId::new(0);
    let cases = [
        (slippery_bogle(), KeywordSet::TRAMPLE),
        (mogg_fanatic(), KeywordSet::HASTE),
        (aven_envoy(), KeywordSet::FIRST_STRIKE),
        (ana_disciple(), KeywordSet::FLYING),
        (gray_ogre(), KeywordSet::EMPTY),
    ];
    for (creature, granted) in cases {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[tribal_golem(), creature])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let golem = on_battlefield(&engine, p0, tribal_golem()).expect("seated");
        assert_eq!(pt(&engine, golem), (4, 4));
        assert_eq!(keywords(&engine, golem), granted);
    }
}

/// All four at once, and none of them for creatures on the other side of the
/// table ("as long as you control").
#[test]
fn tribal_golem_counts_only_my_creatures() {
    let p0 = PlayerId::new(0);
    let tribe = [
        slippery_bogle(),
        mogg_fanatic(),
        aven_envoy(),
        ana_disciple(),
    ];
    let mut mine = vec![tribal_golem()];
    mine.extend(tribe);
    let mut engine = Duel::new(SEED, forest()).battlefield(0, &mine).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let golem = on_battlefield(&engine, p0, tribal_golem()).expect("seated");
    assert_eq!(
        keywords(&engine, golem),
        KeywordSet::TRAMPLE
            .union(KeywordSet::HASTE)
            .union(KeywordSet::FIRST_STRIKE)
            .union(KeywordSet::FLYING)
    );

    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tribal_golem()])
        .battlefield(1, &tribe)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let golem = on_battlefield(&engine, p0, tribal_golem()).expect("seated");
    assert_eq!(keywords(&engine, golem), KeywordSet::EMPTY);
}

/// The Zombie clause grants "{B}: Regenerate this creature": with a Zombie
/// the shield can be bought and saves it; without one the ability is absent
/// even with the Swamp untapped.
#[test]
fn tribal_golem_regenerates_while_i_control_a_zombie() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tribal_golem(), carrion_feeder(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let golem = on_battlefield(&engine, p0, tribal_golem()).expect("seated");
    tap_all_mana(&mut engine, p0);
    raise_a_shield(&mut engine, p0, golem, crate::choice::GRANTED_ABILITY);
    kill(&mut engine, golem);
    assert_eq!(
        engine.state().object(golem).map(|o| o.zone),
        Some(Zone::Battlefield),
        "the granted regeneration saved it"
    );

    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tribal_golem(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let golem = on_battlefield(&engine, p0, tribal_golem()).expect("seated");
    tap_all_mana(&mut engine, p0);
    assert!(
        !priority_offer(&engine)
            .abilities
            .contains(&(golem, crate::choice::GRANTED_ABILITY)),
        "no Zombie, no regeneration"
    );
}
