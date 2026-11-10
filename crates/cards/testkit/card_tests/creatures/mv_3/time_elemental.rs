//! Printed card behavior through the real engine.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn elemental_cannot_bounce_enchanted_permanents() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::TIME_ELEMENTAL, grizzly_bears(), forest()])
        .hand(0, &[index::HOLY_STRENGTH])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let bear = on_battlefield(&e, p0, grizzly_bears()).unwrap();
    let land = on_battlefield(&e, p0, forest()).unwrap();
    attach_test_aura(&mut e, p0, bear);
    give_mana(&mut e, p0, ManaColor::Blue, 4);
    activate(&mut e, p0, index::TIME_ELEMENTAL, 2);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("target")
    };
    assert!(!options.contains(&bear));
    assert!(options.contains(&land));
    aim_at(&mut e, p0, land);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(land).unwrap().zone, Zone::Hand);
}
#[test]
fn elemental_blocking_schedules_sacrifice_and_damage_at_end_of_combat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[llanowar_elves()])
        .battlefield(1, &[index::TIME_ELEMENTAL])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let elf = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    let elemental = on_battlefield(&e, p1, index::TIME_ELEMENTAL).unwrap();
    attack_and_collect_blocks(&mut e, elf, p1);
    e.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(elemental, elf)],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(e.state().object(elemental).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state().players[1].life, 15);
}

/// "When this creature attacks or blocks, at end of combat, sacrifice it and
/// it deals 5 damage to you." The attack half: it survives combat damage
/// itself (it has 0 power) and is sacrificed only at the end of combat, and
/// the 5 damage goes to its controller, not to the defending player.
#[test]
fn elemental_attacking_schedules_sacrifice_and_damage_at_end_of_combat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::TIME_ELEMENTAL])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let elemental = on_battlefield(&e, p0, index::TIME_ELEMENTAL).unwrap();
    attack_and_collect_blocks(&mut e, elemental, p1);
    e.apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    assert_eq!(
        e.state().object(elemental).unwrap().zone,
        Zone::Battlefield,
        "nothing is sacrificed until the end of combat"
    );
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(e.state().object(elemental).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state().players[0].life, 15, "5 damage to its controller");
    assert_eq!(e.state().players[1].life, 20, "and none to the defender");
}
