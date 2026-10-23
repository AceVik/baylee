//! Prophecy — `{W}` Sorcery (Homelands)
//!
//! Reveal the top card of target opponent's library. If it's a land, you gain
//! 1 life. Then that player shuffles.
//! Draw a card at the beginning of the next turn's upkeep.
//!
//! (`Coverage::Partial`: revealing the top card of target opponent's library
//! to gain 1 life if it is a land is not in `Effect`; target opponent shuffles
//! their library and you draw a card at the beginning of the next turn's
//! upkeep.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn prophecy() -> CardIndex {
    card_index("8dcf15fd-f000-4424-b74c-22fc2642b8c3")
}

#[test]
fn prophecy_shuffles_target_opponent_library_and_draws_on_next_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[prophecy()])
        .battlefield(0, &[plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let spell = in_hand(&engine, p0, prophecy()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    assert!(
        engine.apply(p0, PlayerAction::ChoosePlayer(p0)).is_err(),
        "target opponent: you are not one"
    );
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}
