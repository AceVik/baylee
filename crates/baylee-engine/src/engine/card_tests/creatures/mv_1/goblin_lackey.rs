#[allow(clippy::wildcard_imports)]
use super::*;

fn goblin_lackey() -> CardIndex {
    card_index("213fbbdd-f1bc-4bba-9124-7b26e9a706c8")
}

fn goblin_patrol() -> CardIndex {
    card_index("edf25cbe-fa28-44ca-b4be-1d2312e4f6ac")
}

#[test]
fn goblin_lackey_puts_goblin_from_hand_onto_battlefield_on_player_damage() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[goblin_patrol()])
        .battlefield(0, &[goblin_lackey()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lackey = on_battlefield(&engine, p0, goblin_lackey()).expect("seated");
    let patrol = in_hand(&engine, p0, goblin_patrol()).expect("in hand");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(lackey, baylee_core::ids::Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo { .. } | Pending::ChooseCards { .. }
        ) || on_battlefield(e, p0, goblin_patrol()).is_some()
    });
    if matches!(engine.pending(), Pending::YesNo { .. }) {
        engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    }
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![patrol],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, goblin_patrol()).is_some());
}

#[test]
fn goblin_lackey_triggers_on_noncombat_player_damage_and_can_put_nothing() {
    use crate::engine::card_tests::legends_kit as lk;
    use baylee_core::generated::index;
    for accept in [false, true] {
        let p0 = PlayerId::new(0);
        let p1 = PlayerId::new(1);
        let mut engine = lk::game(
            SEED,
            &[goblin_lackey()],
            &[],
            &[index::HERMETIC_STUDY, goblin_patrol(), grizzly_bears()],
            &[],
        );
        let lackey = on_battlefield(&engine, p0, goblin_lackey()).unwrap();
        let patrol = in_hand(&engine, p0, goblin_patrol()).unwrap();
        let bear = in_hand(&engine, p0, grizzly_bears()).unwrap();
        lk::float(&mut engine, p0, &[(ManaColor::Blue, 2)]);
        lk::cast_at(&mut engine, p0, index::HERMETIC_STUDY, &[lackey], &[]);
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: lackey,
                    ability_index: crate::choice::GRANTED_ABILITY,
                },
            )
            .unwrap();
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p1],
                },
            )
            .unwrap();
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        let Pending::ChooseCards { options, min, .. } = engine.pending().clone() else {
            unreachable!()
        };
        assert_eq!(min, 0);
        assert!(options.contains(&patrol));
        assert!(!options.contains(&bear));
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: if accept { vec![patrol] } else { vec![] },
                },
            )
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            on_battlefield(&engine, p0, goblin_patrol()).is_some(),
            accept
        );
        assert_eq!(in_hand(&engine, p0, goblin_patrol()).is_some(), !accept);
        assert!(in_hand(&engine, p0, grizzly_bears()).is_some());
    }
}
