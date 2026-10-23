//! `cards/creatures/mv_1/carnophage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

const P0: PlayerId = PlayerId::new(0);

fn carnophage() -> CardIndex {
    card_index("9ac77cfd-7d78-4c8a-bffc-17649a852114")
}

/// Carnophage (`9ac77cfd-7d78-4c8a-bffc-17649a852114`) — {B} Creature — Zombie (Implemented):
/// "At the beginning of your upkeep, tap this creature unless you pay 1 life."
#[test]
fn carnophage_upkeep_pays_one_life_or_taps_itself() {
    for pay in [false, true] {
        let mut engine = Duel::new(SEED, swamp())
            .battlefield(0, &[carnophage()])
            .start();
        keep_mulligans(&mut engine);

        let c = on_battlefield(&engine, P0, carnophage()).unwrap();
        kit::drive(
            &mut engine,
            |e| matches!(e.state().turn.phase, Phase::FirstMain) && e.state().turn.active == P0,
            |_, p| matches!(p, Pending::YesNo { .. }).then_some(PlayerAction::YesNo(pay)),
        );

        assert_eq!(entered_tapped(&engine, c), !pay);
        assert_eq!(engine.state().players[0].life, if pay { 19 } else { 20 });
        reach_their_main_phase(&mut engine, PlayerId::new(1));
        assert_eq!(engine.state().players[0].life, if pay { 19 } else { 20 });
    }
}
