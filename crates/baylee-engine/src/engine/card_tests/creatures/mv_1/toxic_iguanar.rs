//! Oracle checklist: any own green permanent (including a noncreature) grants deathtouch; opposing green and own nongreen permanents do not; losing the last own green permanent removes it.
#[allow(clippy::wildcard_imports)]
use super::*;

fn toxic_iguanar() -> CardIndex {
    card_index("87e148a0-4299-4771-b68a-6652a4c1c535")
}

#[test]
fn iguanar_deathtouch_depends_on_a_green_permanent_you_control() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[toxic_iguanar(), garruk_s_uprising(), sol_ring()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[naturalize()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let iguanar = on_battlefield(&e, p0, toxic_iguanar()).unwrap();
    assert_eq!(pt(&e, iguanar), (1, 1), "the static grants only a keyword");
    assert!(keywords(&e, iguanar).contains(KeywordSet::DEATHTOUCH));

    // Destroy the green noncreature while own nongreen and opposing green
    // permanents remain; neither of those satisfies the condition.
    let green_enchantment = on_battlefield(&e, p0, garruk_s_uprising()).unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Green, 2);
    e.refresh_offer();
    cast_with_floating(&mut e, p0, naturalize());
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("Naturalize target")
    };
    assert!(options.contains(&green_enchantment));
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![green_enchantment],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, p0, sol_ring()).is_some());
    assert!(on_battlefield(&e, p1, llanowar_elves()).is_some());
    assert!(!keywords(&e, iguanar).contains(KeywordSet::DEATHTOUCH));

    // A separate board proves opposing green alone is not enough.
    let mut other = Duel::new(SEED, forest())
        .battlefield(0, &[toxic_iguanar()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut other);
    assert!(walk_to_own_main(&mut other, p0));
    let alone = on_battlefield(&other, p0, toxic_iguanar()).unwrap();
    assert!(!keywords(&other, alone).contains(KeywordSet::DEATHTOUCH));
}

#[test]
fn iguanar_deathtouch_kills_a_larger_blocker_with_one_damage() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[toxic_iguanar(), llanowar_elves()])
        .battlefield(1, &[fangren_hunter()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let iguanar = on_battlefield(&e, p0, toxic_iguanar()).unwrap();
    let hunter = on_battlefield(&e, p1, fangren_hunter()).unwrap();
    reach_their_main_phase(&mut e, p1);
    pass_until(&mut e, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(iguanar, Defender::Player(p1))],
        },
    )
    .unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == p1),
    );
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = e.pending().clone()
    else {
        panic!("defender chooses blockers");
    };
    assert!(
        blockers
            .iter()
            .any(|b| b.blocker == hunter && b.attackers.contains(&iguanar))
    );
    e.apply(
        player,
        PlayerAction::DeclareBlockers {
            blockers: vec![(hunter, iguanar)],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(in_graveyard(&e, p1, fangren_hunter()).is_some());
    assert!(
        in_graveyard(&e, p0, toxic_iguanar()).is_some(),
        "ordinary return damage still kills the 1/1"
    );
}
