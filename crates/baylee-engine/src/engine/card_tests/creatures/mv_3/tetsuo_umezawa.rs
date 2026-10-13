//! Printed card behavior through the real engine.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as kit;
use baylee_core::generated::index;

#[test]
fn tetsuo_refuses_auras_and_destroys_only_tapped_or_blocking_creatures() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[index::TETSUO_UMEZAWA, grizzly_bears(), llanowar_elves()],
        )
        .hand(0, &[index::HOLY_STRENGTH])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let tetsuo = on_battlefield(&e, p0, index::TETSUO_UMEZAWA).unwrap();
    let bear = on_battlefield(&e, p0, grizzly_bears()).unwrap();
    give_mana(&mut e, p0, ManaColor::White, 1);
    let aura = in_hand(&e, p0, index::HOLY_STRENGTH).unwrap();
    e.apply(p0, PlayerAction::CastSpell { card: aura }).unwrap();
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("target")
    };
    assert!(!options.contains(&tetsuo));
    aim_at(&mut e, p0, bear);
    pass_until(&mut e, stack_is_empty);
    let elf = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    activate(&mut e, p0, llanowar_elves(), 0);
    give_mana(&mut e, p0, ManaColor::Blue, 1);
    give_mana(&mut e, p0, ManaColor::Black, 2);
    give_mana(&mut e, p0, ManaColor::Red, 1);
    activate(&mut e, p0, index::TETSUO_UMEZAWA, 0);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("target")
    };
    assert!(options.contains(&elf));
    assert!(!options.contains(&bear));
    aim_at(&mut e, p0, elf);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(elf).unwrap().zone, Zone::Graveyard);
}

/// "can't be the target of AURA spells": any other spell may target him.
#[test]
fn tetsuo_can_be_targeted_by_a_spell_that_is_not_an_aura() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::TETSUO_UMEZAWA])
        .hand(0, &[index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let tetsuo = on_battlefield(&e, p0, index::TETSUO_UMEZAWA).unwrap();
    kit::float(&mut e, p0, &[(ManaColor::Red, 1)]);
    let bolt = in_hand(&e, p0, index::LIGHTNING_BOLT).unwrap();
    e.apply(p0, PlayerAction::CastSpell { card: bolt }).unwrap();
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("the Bolt asks for a target, got {:?}", e.pending())
    };
    assert!(
        options.contains(&tetsuo),
        "only Aura spells are shut out; a Lightning Bolt may target him: {options:?}"
    );
    aim_at(&mut e, p0, tetsuo);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(tetsuo).unwrap().zone,
        Zone::Graveyard,
        "the Bolt really did target and kill the 3/3"
    );
}

/// "Destroy target tapped or blocking creature": an untapped creature that is
/// blocking is a target, an untapped creature that is not, is not.
#[test]
fn tetsuo_may_destroy_an_untapped_blocker_but_not_an_untapped_bystander() {
    let (p0, p1) = (kit::P0, kit::P1);
    let mut e = kit::game(
        SEED,
        &[index::TETSUO_UMEZAWA, index::GRIZZLY_BEARS],
        &[index::LLANOWAR_ELVES, index::GIANT_SPIDER],
        &[],
        &[],
    );
    let tetsuo = kit::obj(&e, p0, index::TETSUO_UMEZAWA);
    let bears = kit::obj(&e, p0, index::GRIZZLY_BEARS);
    let elves = kit::obj(&e, p1, index::LLANOWAR_ELVES);
    let spider = kit::obj(&e, p1, index::GIANT_SPIDER);
    kit::attack(&mut e, &[bears]);
    kit::block(&mut e, &[(elves, bears)]);
    assert!(
        !kit::tapped(&e, elves) && !kit::tapped(&e, spider),
        "neither of the opponent's creatures is tapped"
    );

    kit::float(
        &mut e,
        p0,
        &[
            (ManaColor::Blue, 1),
            (ManaColor::Black, 2),
            (ManaColor::Red, 1),
        ],
    );
    activate(&mut e, p0, index::TETSUO_UMEZAWA, 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("the ability asks for a target, got {:?}", e.pending())
    };
    assert!(
        options.contains(&elves),
        "an untapped creature that is blocking is a legal target: {options:?}"
    );
    assert!(
        options.contains(&bears),
        "and so is the attacker, which tapped to attack: {options:?}"
    );
    assert!(
        !options.contains(&spider),
        "an untapped creature that blocks nothing is not: {options:?}"
    );
    assert!(
        !options.contains(&tetsuo),
        "neither is Tetsuo, who is untapped until the cost is paid: {options:?}"
    );
    aim_at(&mut e, p0, elves);
    assert!(kit::tapped(&e, tetsuo), "{{T}} is part of the cost");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(kit::zone_of(&e, elves), Zone::Graveyard);
    assert_eq!(kit::zone_of(&e, spider), Zone::Battlefield);
}

/// "target tapped or blocking CREATURE", whoever controls it: a tapped
/// creature of the opponent's is a target, a tapped land and a tapped
/// artifact are not.
#[test]
fn tetsuo_may_destroy_an_opponents_tapped_creature_but_no_tapped_noncreature() {
    let (p0, p1) = (kit::P0, kit::P1);
    let mut e = kit::game(
        SEED,
        &[index::TETSUO_UMEZAWA],
        &[
            index::LLANOWAR_ELVES,
            index::GIANT_SPIDER,
            index::SOL_RING,
            index::FOREST,
        ],
        &[],
        &[],
    );
    let elves = kit::obj(&e, p1, index::LLANOWAR_ELVES);
    let spider = kit::obj(&e, p1, index::GIANT_SPIDER);
    let ring = kit::obj(&e, p1, index::SOL_RING);
    let land = kit::obj(&e, p1, index::FOREST);
    for tapped in [elves, ring, land] {
        e.dev_state_mut(p0).unwrap().set_tapped(tapped, true);
    }
    kit::float(
        &mut e,
        p0,
        &[
            (ManaColor::Blue, 1),
            (ManaColor::Black, 2),
            (ManaColor::Red, 1),
        ],
    );
    activate(&mut e, p0, index::TETSUO_UMEZAWA, 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("the ability asks for a target, got {:?}", e.pending())
    };
    assert!(
        options.contains(&elves),
        "a tapped creature controlled by the opponent: {options:?}"
    );
    assert!(
        !options.contains(&spider),
        "an untapped creature that is not blocking: {options:?}"
    );
    assert!(
        !options.contains(&ring) && !options.contains(&land),
        "a tapped artifact and a tapped land are not creatures: {options:?}"
    );
}

/// "{U}{B}{B}{R}, {T}": all four mana and Tetsuo's own tap.
#[test]
fn tetsuo_needs_his_whole_cost_to_destroy() {
    let (p0, p1) = (kit::P0, kit::P1);
    let mut e = kit::game(
        SEED,
        &[index::TETSUO_UMEZAWA],
        &[index::LLANOWAR_ELVES, index::GIANT_SPIDER],
        &[],
        &[],
    );
    let tetsuo = kit::obj(&e, p0, index::TETSUO_UMEZAWA);
    for victim in [index::LLANOWAR_ELVES, index::GIANT_SPIDER] {
        let id = kit::obj(&e, p1, victim);
        e.dev_state_mut(p0).unwrap().set_tapped(id, true);
    }
    kit::float(
        &mut e,
        p0,
        &[
            (ManaColor::Blue, 1),
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
        ],
    );
    assert!(
        !kit::offered(&e, index::TETSUO_UMEZAWA, p0, 0),
        "{{U}}{{B}}{{R}} is a mana short of {{U}}{{B}}{{B}}{{R}}"
    );
    kit::float(&mut e, p0, &[(ManaColor::Black, 1)]);
    assert!(kit::offered(&e, index::TETSUO_UMEZAWA, p0, 0));

    activate(&mut e, p0, index::TETSUO_UMEZAWA, 0);
    let victim = kit::obj(&e, p1, index::LLANOWAR_ELVES);
    kit::aim(&mut e, &[victim], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(kit::pool_total(&e, p0), 0, "all four mana were paid");
    assert!(kit::tapped(&e, tetsuo));
    kit::float(
        &mut e,
        p0,
        &[
            (ManaColor::Blue, 1),
            (ManaColor::Black, 2),
            (ManaColor::Red, 1),
        ],
    );
    assert!(
        !kit::offered(&e, index::TETSUO_UMEZAWA, p0, 0),
        "a tapped Tetsuo cannot pay {{T}} again, with a tapped creature still to destroy"
    );
}
