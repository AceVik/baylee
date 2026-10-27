//! Fatespinner — `{1}{U}{U}` 1/2 Creature — Human Wizard (MRD).
//! Oracle: "At the beginning of each opponent's upkeep, that player chooses draw step, main phase, or combat phase. The player skips each instance of the chosen step or phase this turn."

use super::*;

fn fatespinner() -> CardIndex {
    card_index("676935c8-22e0-427f-9566-16778cea0873")
}

#[test]
fn fatespinner_makes_opponent_choose_and_skip_draw_step_on_their_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let spinner = fatespinner();
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[spinner])
        .hand(1, &[island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    // Choice 0 = draw step.
    engine.apply(p1, PlayerAction::ChooseNumber(0)).unwrap();
    reach_their_main_phase(&mut engine, p1);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        hand_before,
        "p1 skipped their draw step this turn"
    );
}
