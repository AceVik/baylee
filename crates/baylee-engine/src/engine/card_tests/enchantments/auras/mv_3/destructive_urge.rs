#[allow(clippy::wildcard_imports)]
use super::*;

fn destructive_urge() -> CardIndex {
    card_index("5ab62b09-1135-4d22-866b-3a0d64228bf9")
}

#[test]
fn destructive_urge_makes_damaged_player_sacrifice_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[destructive_urge()])
        .battlefield(0, &[mountain(), mountain(), mountain(), grizzly_bears()])
        .battlefield(1, &[plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    let opp_land = on_battlefield(&engine, p1, plains()).expect("seated");
    cast_from_hand(&mut engine, p0, destructive_urge());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        aim_at(&mut engine, p0, bear);
    }
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, baylee_core::ids::Defender::Player(p1))],
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
        matches!(e.pending(), Pending::ChooseCards { .. })
            || on_battlefield(e, p1, plains()).is_none()
    });
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p1,
                PlayerAction::ChooseObjects {
                    objects: vec![opp_land],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, plains()).is_none());
}

#[test]
fn destructive_urge_does_not_trigger_on_noncombat_damage() {
    use crate::engine::card_tests::legends_kit as lk;
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(
        SEED,
        &[grizzly_bears()],
        &[plains()],
        &[destructive_urge(), index::HERMETIC_STUDY],
        &[],
    );
    let host = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    lk::float(
        &mut engine,
        p0,
        &[(ManaColor::Red, 3), (ManaColor::Blue, 2)],
    );
    lk::cast_at(&mut engine, p0, destructive_urge(), &[host], &[]);
    lk::cast_at(&mut engine, p0, index::HERMETIC_STUDY, &[host], &[]);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: host,
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
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 19);
    assert!(on_battlefield(&engine, p1, plains()).is_some());
}
