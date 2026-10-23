//! `cards/creatures/mv_6/warrior_angel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
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
