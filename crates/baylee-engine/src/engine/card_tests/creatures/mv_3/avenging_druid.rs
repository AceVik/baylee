//! `cards/creatures/mv_3/avenging_druid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn avenging_druid() -> CardIndex {
    card_index("1f3fb367-60b5-4b36-9604-3064afdac0d9")
}

/// Avenging Druid (`1f3fb367-60b5-4b36-9604-3064afdac0d9`) — {2}{G} Creature — Human Druid (Partial):
/// "Whenever this creature deals damage to an opponent, you may reveal cards from the top of your library until you reveal a land card. If you do, put that card onto the battlefield and put all other cards revealed this way into your graveyard."
#[test]
fn avenging_druid_combat_damage_to_opponent_puts_revealed_land_onto_battlefield() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[avenging_druid()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let druid = on_battlefield(&engine, P0, avenging_druid()).unwrap();
    kit::declare_attack(&mut engine, &[druid], P1);
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });

    assert_eq!(engine.state().players[1].life, 19);
    assert!(on_battlefield(&engine, P0, forest()).is_some());
}
