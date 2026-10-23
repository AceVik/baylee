//! `cards/creatures/mv_6/warrior_angel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::engine::card_tests::partials_w1b_kit as kit;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn warrior_angel() -> CardIndex {
    card_index("97140551-341a-40a1-93fe-0fcaa0c3be33")
}

/// Warrior Angel (`97140551-341a-40a1-93fe-0fcaa0c3be33`) — {4}{W}{W} Creature — Angel Warrior (Implemented):
/// "Flying. Whenever this creature deals damage, you gain that much life."
#[test]
fn warrior_angel_gains_life_equal_to_damage_dealt() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[warrior_angel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let angel = on_battlefield(&engine, P0, warrior_angel()).unwrap();
    kit::declare_attack(&mut engine, &[angel], P1);
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });

    assert_eq!(engine.state().players[1].life, 17);
    assert_eq!(engine.state().players[0].life, 23);
}

#[test]
fn creature_damage_also_gains_life_and_ground_creatures_cannot_block() {
    use baylee_core::generated::index;
    let mut e = lk::game(
        SEED,
        &[warrior_angel()],
        &[index::AIR_ELEMENTAL, index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let angel = lk::obj(&e, P0, warrior_angel());
    let air = lk::obj(&e, P1, index::AIR_ELEMENTAL);
    let bear = lk::obj(&e, P1, index::GRIZZLY_BEARS);
    lk::attackers_offered(&mut e);
    let options = lk::declare(&mut e, &[angel]);
    assert!(lk::may_block(&options, air, angel));
    assert!(!lk::may_block(&options, bear, angel));
    lk::block(&mut e, &[(air, angel)]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, P0), 23);
    assert_eq!(lk::life(&e, P1), 20);
}
