//! `house-foresight`: a game this engine builds has the house's chairs try
//! their options in forks before they answer; without the feature it never
//! does (`Session::set_foresight` is off by default, and the method does not
//! exist without `simulation`).

use super::*;
use baylee_core::preset::{AIProfile, SeatController};

/// Both chairs `expert` at the newest generation (a foresight tier of its
/// own), set up as the gateway orders a game, and played one house answer
/// a slice until a chair has searched: it does, as soon as it has a
/// question with more than one sensible answer.
#[test]
fn the_house_searches_at_every_game_this_engine_builds() {
    let mut preset = duel(0);
    for seat in &mut preset.seats {
        seat.controller = SeatController::Ai(AIProfile::EXPERT);
    }
    assert!(AIProfile::EXPERT.foresight().is_some(), "expert has a tier");
    let mut runner = EngineRunner::new().with_pump_budget(1);
    setup(&mut runner, &preset);
    let searched = |runner: &EngineRunner| {
        runner
            .session()
            .map_or(0, |session| session.foresight_stats().searched)
    };
    for _ in 0..2_000 {
        if searched(&runner) > 0 || !runner.needs_pump() {
            break;
        }
        runner.pump_slice(&[]);
    }
    assert!(
        searched(&runner) > 0,
        "no house answer was searched: {:?}",
        runner.session().map(Session::foresight_stats)
    );
}
