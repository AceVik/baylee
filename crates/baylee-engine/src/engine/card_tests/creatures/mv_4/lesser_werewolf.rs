//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as kit;
use crate::turn::Step;
use baylee_core::generated::index;

#[test]
fn lesser_werewolf_checks_power_on_resolution_and_shrinks_only_its_combat_partner() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::LESSER_WEREWOLF])
        .battlefield(1, &[giant_spider(), grizzly_bears()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let wolf = on_battlefield(&e, p0, index::LESSER_WEREWOLF).unwrap();
    let spider = on_battlefield(&e, p1, giant_spider()).unwrap();
    attack_and_collect_blocks(&mut e, wolf, p1);
    e.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(spider, wolf)],
        },
    )
    .unwrap();
    for _ in 0..3 {
        super::super::completion_helpers::give_mana(&mut e, p0, ManaColor::Black, 1);
        activate(&mut e, p0, index::LESSER_WEREWOLF, 0);
        let Pending::ChooseTargets { options, .. } = e.pending() else {
            panic!("target")
        };
        assert_eq!(options, &vec![spider]);
        aim_at(&mut e, p0, spider);
    }
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, wolf), (0, 4));
    assert_eq!(pt(&e, spider), (2, 2));
    assert_eq!(
        counters_on(
            &e,
            spider,
            CounterKind::Minus {
                power: 0,
                toughness: 1
            }
        ),
        2
    );
    // The third activation was legal but does nothing once power reaches zero.
    reach_their_main_phase(&mut e, p1);
    assert_eq!(pt(&e, wolf), (2, 4));
    assert_eq!(pt(&e, spider), (2, 2));
    super::super::completion_helpers::give_mana(&mut e, p0, ManaColor::Black, 1);
    e.apply(p1, PlayerAction::PassPriority).unwrap();
    assert!(
        e.apply(
            p0,
            PlayerAction::ActivateAbility {
                source: wolf,
                ability_index: 0
            }
        )
        .is_err()
    );
}

/// "Activate only during the declare blockers step", and "{B}": the ability is
/// offered there with a Giant Spider in the way and a black mana to pay, but
/// not without the mana, and not in the combat damage or end of combat steps,
/// where the Spider still blocks the Werewolf and would be a legal target.
#[test]
fn lesser_werewolf_is_offered_only_in_the_declare_blockers_step_and_for_a_black_mana() {
    let (p0, p1) = (kit::P0, kit::P1);
    let mut e = kit::game(
        SEED,
        &[index::LESSER_WEREWOLF],
        &[index::GIANT_SPIDER],
        &[],
        &[],
    );
    let wolf = kit::obj(&e, p0, index::LESSER_WEREWOLF);
    let spider = kit::obj(&e, p1, index::GIANT_SPIDER);
    kit::attack(&mut e, &[wolf]);
    kit::block(&mut e, &[(spider, wolf)]);
    assert_eq!(e.state().turn.step, Step::DeclareBlockers);
    assert!(
        !kit::offered(&e, index::LESSER_WEREWOLF, p0, 0),
        "{{B}}: with nothing floating it cannot be paid"
    );
    kit::float(&mut e, p0, &[(ManaColor::Black, 1)]);
    assert!(
        kit::offered(&e, index::LESSER_WEREWOLF, p0, 0),
        "declare blockers, a black mana, a blocker to target: offered"
    );

    kit::to_step(&mut e, p0, Step::CombatDamage);
    assert_eq!(
        kit::zone_of(&e, spider),
        Zone::Battlefield,
        "a 2/4 Wolf does not kill a 2/4 Spider, which is still blocking it"
    );
    kit::float(&mut e, p0, &[(ManaColor::Black, 1)]);
    assert!(
        !kit::offered(&e, index::LESSER_WEREWOLF, p0, 0),
        "the combat damage step is not the declare blockers step"
    );
    kit::to_step(&mut e, p0, Step::CombatEnd);
    kit::float(&mut e, p0, &[(ManaColor::Black, 1)]);
    assert!(
        !kit::offered(&e, index::LESSER_WEREWOLF, p0, 0),
        "nor is the end of combat step"
    );
}

/// "a creature blocking OR BLOCKED BY this creature": when the Werewolf is
/// the blocker, the attacker it blocks is the target.
#[test]
fn lesser_werewolf_may_target_the_attacker_it_is_blocking() {
    let (p0, p1) = (kit::P0, kit::P1);
    let mut e = kit::game(
        SEED,
        &[index::LESSER_WEREWOLF],
        &[index::GIANT_SPIDER],
        &[],
        &[],
    );
    let wolf = kit::obj(&e, p0, index::LESSER_WEREWOLF);
    let spider = kit::obj(&e, p1, index::GIANT_SPIDER);
    reach_their_main_phase(&mut e, p1);
    kit::attack(&mut e, &[spider]);
    kit::block(&mut e, &[(wolf, spider)]);
    kit::to_step(&mut e, p0, Step::DeclareBlockers);
    kit::float(&mut e, p0, &[(ManaColor::Black, 1)]);
    activate(&mut e, p0, index::LESSER_WEREWOLF, 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("the ability asks for a target, got {:?}", e.pending())
    };
    assert_eq!(
        options,
        vec![spider],
        "the attacker the Werewolf blocks is the only creature in its combat"
    );
    aim_at(&mut e, p0, spider);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, wolf), (1, 4), "the Werewolf gets -1/-0");
    assert_eq!(
        counters_on(
            &e,
            spider,
            CounterKind::Minus {
                power: 0,
                toughness: 1
            }
        ),
        1,
        "and the attacker gets the -0/-1 counter"
    );
}

/// "blocking or blocked by THIS creature": a creature that blocks another of
/// the Werewolf's side is not in the Werewolf's combat.
#[test]
fn lesser_werewolf_cannot_target_a_creature_that_blocks_someone_else() {
    let (p0, p1) = (kit::P0, kit::P1);
    let mut e = kit::game(
        SEED,
        &[index::LESSER_WEREWOLF, index::GRIZZLY_BEARS],
        &[index::GIANT_SPIDER, index::LLANOWAR_ELVES],
        &[],
        &[],
    );
    let wolf = kit::obj(&e, p0, index::LESSER_WEREWOLF);
    let bears = kit::obj(&e, p0, index::GRIZZLY_BEARS);
    let spider = kit::obj(&e, p1, index::GIANT_SPIDER);
    let elves = kit::obj(&e, p1, index::LLANOWAR_ELVES);
    kit::attack(&mut e, &[wolf, bears]);
    kit::block(&mut e, &[(spider, wolf), (elves, bears)]);
    kit::float(&mut e, p0, &[(ManaColor::Black, 1)]);
    activate(&mut e, p0, index::LESSER_WEREWOLF, 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("the ability asks for a target, got {:?}", e.pending())
    };
    assert_eq!(
        options,
        vec![spider],
        "the Spider blocks the Werewolf; the Elves block the Bears, and the \
         Bears are blocked by the Elves, not by the Werewolf"
    );
}
