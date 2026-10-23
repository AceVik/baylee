#[allow(clippy::wildcard_imports)]
use super::*;

fn arc_lightning() -> CardIndex {
    card_index("5acc8b39-3c3e-4012-8cfd-ac3c2c4ca982")
}

#[test]
fn arc_lightning_deals_three_damage_divided_among_targets() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[arc_lightning()])
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, arc_lightning());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p1],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 17);
}

#[test]
fn arc_lightning_divides_positive_damage_between_two_or_three_distinct_targets() {
    use crate::choice::{NumberPrompt, TargetRef};
    use baylee_core::generated::index;
    for three in [false, true] {
        let p0 = PlayerId::new(0);
        let p1 = PlayerId::new(1);
        let mut engine = Duel::new(SEED, mountain())
            .hand(0, &[arc_lightning()])
            .battlefield(0, &[mountain(), mountain(), mountain()])
            .battlefield(1, &[index::WALL_OF_STONE, index::WALL_OF_AIR])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let a = on_battlefield(&engine, p1, index::WALL_OF_STONE).unwrap();
        let b = on_battlefield(&engine, p1, index::WALL_OF_AIR).unwrap();
        cast_from_hand(&mut engine, p0, arc_lightning());
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![]
                    }
                )
                .is_err()
        );
        let objects = if three { vec![a, b] } else { vec![a] };
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects,
                    players: vec![p0],
                },
            )
            .unwrap();
        while let Pending::ChooseNumber {
            reason: NumberPrompt::DivideDamage { target, .. },
            ..
        } = engine.pending().clone()
        {
            let share = if !three
                && matches!(target, TargetRef::Object(identity) if identity.object == a)
            {
                2
            } else {
                1
            };
            assert!(engine.apply(p0, PlayerAction::ChooseNumber(0)).is_err());
            engine.apply(p0, PlayerAction::ChooseNumber(share)).unwrap();
        }
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            engine.state().object(a).unwrap().damage,
            if three { 1 } else { 2 }
        );
        assert_eq!(engine.state().object(b).unwrap().damage, u16::from(three));
        assert_eq!(engine.state().players[0].life, 19);
        assert_eq!(engine.state().players[1].life, 20);
    }
}
