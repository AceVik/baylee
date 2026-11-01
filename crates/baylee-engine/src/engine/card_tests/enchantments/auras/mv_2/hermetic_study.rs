#[allow(clippy::wildcard_imports)]
use super::*;

fn hermetic_study() -> CardIndex {
    card_index("792d7818-ee5c-4254-b969-7b50f475c629")
}

#[test]
fn hermetic_study_grants_tap_ping_ability_to_enchanted_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[hermetic_study()])
        .battlefield(0, &[island(), island(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    cast_from_hand(&mut engine, p0, hermetic_study());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        aim_at(&mut engine, p0, bear);
    }
    pass_until(&mut engine, stack_is_empty);

    activate(
        &mut engine,
        p0,
        grizzly_bears(),
        crate::choice::GRANTED_ABILITY,
    );
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

    assert_eq!(engine.state().players[1].life, 19);
}

#[test]
fn hermetic_study_grants_the_ability_to_an_opponents_creature_and_its_controller() {
    use crate::engine::card_tests::legends_kit as lk;
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(
        SEED,
        &[grizzly_bears()],
        &[grizzly_bears()],
        &[hermetic_study()],
        &[],
    );
    let host = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    lk::float(&mut engine, p0, &[(ManaColor::Blue, 2)]);
    lk::cast_at(&mut engine, p0, hermetic_study(), &[host], &[]);
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: host,
                    ability_index: crate::choice::GRANTED_ABILITY
                }
            )
            .is_err()
    );
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: host,
                ability_index: crate::choice::GRANTED_ABILITY,
            },
        )
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 19);
    assert!(lk::tapped(&engine, host));
}
