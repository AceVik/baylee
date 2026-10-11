//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn relic_bind_chooses_its_mode_and_target_when_the_trigger_is_stacked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for mode in [0, 1] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::ISLAND, index::ISLAND, index::ISLAND])
            .battlefield(1, &[index::MIND_STONE])
            .hand(0, &[index::RELIC_BIND])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let stone = on_battlefield(&e, p1, index::MIND_STONE).unwrap();
        cast_from_hand(&mut e, p0, index::RELIC_BIND);
        aim_at(&mut e, p0, stone);
        pass_until(&mut e, stack_is_empty);
        pass_until(
            &mut e,
            |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
        );
        activate(&mut e, p1, index::MIND_STONE, 0);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCastMode { .. })
        });
        e.apply(p0, PlayerAction::ChooseMode(mode)).unwrap();
        assert!(matches!(e.pending(), Pending::ChooseTargets { .. }));
        e.apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().players[1].life, if mode == 0 { 19 } else { 21 });
    }
}
