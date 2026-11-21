//! `cards/artifacts/equipment/mv_2/sparring_collar.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn sparring_collar() -> CardIndex {
    card_index("81bfbc5a-c448-4124-ac25-79bd0944136d")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Sparring Collar (81bfbc5a-c448-4124-ac25-79bd0944136d) — {2} Equipment:
/// "Equipped creature has first strike."
/// "{R}{R}: Attach this Equipment to target creature you control."
/// "Equip {1}"
///
/// Equip {1} grants First Strike: a 2/1 Goblin Piker kills a blocking 2/2
/// Grizzly Bears in first-strike combat damage and survives.
#[test]
fn sparring_collar_grants_first_strike_winning_combat_against_bear() {
    let p1 = PlayerId::new(1);
    let mut r = rig_vs(
        sparring_collar(),
        goblin_piker(),
        1,
        &[],
        &[grizzly_bears()],
        &[],
    );
    r.equip(1);
    assert!(keywords(&r.engine, r.host).contains(KeywordSet::FIRST_STRIKE));
    assert!(!keywords(&r.engine, r.bystander).contains(KeywordSet::FIRST_STRIKE));

    let _ = r.attack_to_the_blocks();
    r.engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(r.theirs, r.host)],
            },
        )
        .unwrap();
    pass_until(&mut r.engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });

    assert_eq!(
        r.engine.state().object(r.theirs).unwrap().zone,
        Zone::Graveyard,
        "2/2 blocker died to first-strike damage"
    );
    assert_eq!(
        r.engine.state().object(r.host).unwrap().zone,
        Zone::Battlefield,
        "2/1 First Strike attacker survived with 0 damage"
    );
    assert_eq!(r.engine.state().object(r.host).unwrap().damage, 0);
}

/// "{R}{R}: Attach this Equipment to target creature you control" activates at
/// instant speed on the opponent's turn and moves First Strike to the new host.
#[test]
fn sparring_collar_rr_attaches_at_instant_speed_on_opponents_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                sparring_collar(),
                goblin_piker(),
                jasmine_boreal(),
                forest(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let collar = on_battlefield(&engine, p0, sparring_collar()).unwrap();
    let piker = on_battlefield(&engine, p0, goblin_piker()).unwrap();
    let jasmine = on_battlefield(&engine, p0, jasmine_boreal()).unwrap();
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();

    let my_forest = on_battlefield(&engine, p0, forest()).unwrap();
    tap_mana_where(&mut engine, p0, |id| id == my_forest);
    activate(&mut engine, p0, sparring_collar(), 0);
    aim_at(&mut engine, p0, piker);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, piker).contains(KeywordSet::FIRST_STRIKE));

    reach_their_main_phase(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);

    assert!(
        !ability_offered(&engine, collar, 0),
        "Equip {{1}} is sorcery-speed only"
    );
    assert!(
        priority_offer(&engine).abilities.contains(&(collar, 2)),
        "{{R}}{{R}} attach is instant-speed on opponent's turn"
    );

    activate(&mut engine, p0, sparring_collar(), 2);
    let menu = aim_at(&mut engine, p0, jasmine);
    assert!(menu.contains(&jasmine) && menu.contains(&piker) && !menu.contains(&their_elf));
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, jasmine).contains(KeywordSet::FIRST_STRIKE));
    assert!(!keywords(&engine, piker).contains(KeywordSet::FIRST_STRIKE));
}
