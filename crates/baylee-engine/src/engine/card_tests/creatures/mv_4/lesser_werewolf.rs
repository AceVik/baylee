//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
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
