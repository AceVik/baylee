//! `cards/creatures/mv_6/exalted_dragon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn exalted_dragon() -> CardIndex {
    card_index("c901805e-c196-4b9d-8d5a-e17367fb37bc")
}

/// Exalted Dragon (`c901805e-c196-4b9d-8d5a-e17367fb37bc`) — {4}{W}{W} Creature — Dragon (Implemented):
/// "Flying. This creature can't attack unless you sacrifice a land. (This cost is paid as attackers are declared.)"
#[test]
fn exalted_dragon_cant_attack_unless_controller_sacrifices_a_land() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[exalted_dragon(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let dragon = on_battlefield(&engine, P0, exalted_dragon()).unwrap();
    let land = on_battlefield(&engine, P0, plains()).unwrap();

    kit::declare_attack(&mut engine, &[dragon], P1);
    kit::drive(
        &mut engine,
        |e| matches!(e.state().turn.phase, Phase::SecondMain),
        |_, p| kit::choose_cards(p, &[land]),
    );

    assert!(on_battlefield(&engine, P0, plains()).is_none());
    assert!(in_graveyard(&engine, P0, plains()).is_some());
    assert_eq!(engine.state().players[1].life, 15);
}

#[test]
fn exalted_dragon_cannot_attack_without_a_land_of_its_controllers_own() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut e = lk::game(
        SEED,
        &[exalted_dragon(), gray_ogre()],
        &[plains()],
        &[],
        &[],
    );
    let dragon = lk::obj(&e, P0, exalted_dragon());
    let ogre = lk::obj(&e, P0, gray_ogre());
    let offered = lk::attackers_offered(&mut e);
    assert!(!offered.contains(&dragon));
    assert!(offered.contains(&ogre));
}
