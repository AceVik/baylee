//! "That player skips their next combat phase" (Blinding Angel,
//! `Effect::SkipNextCombatPhase`): one waiting skip per effect, used by the
//! next combat phase that would happen (CR 614.10a). A combat phase another
//! effect already skips ("skip each combat phase this turn") does not use it.

use super::testkit::{Duel, card_index, keep_mulligans, pass_until, reach_their_main_phase};
use super::*;
use baylee_core::ids::CardIndex;

const SEED: u64 = 0x5c1b_c0b1;

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

fn combats_to_skip(engine: &Engine<super::testkit::RegistryLookup>, seat: PlayerId) -> u32 {
    engine.state().players[usize::from(seat.get())].combats_to_skip
}

#[test]
fn a_combat_already_skipped_leaves_the_next_combat_skip_waiting() {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest()).start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let turn = engine.state().turn.number;
    {
        let state = engine.dev_state_mut(PlayerId::new(0)).expect("dev board");
        state.players[1].combats_to_skip = 1;
        state.per_turn.skip_combat_phases = true;
    }
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::SecondMain
    });
    assert_eq!(engine.state().turn.number, turn);
    assert_eq!(
        combats_to_skip(&engine, p1),
        1,
        "this turn's combat was skipped by the other effect"
    );
    // Its next turn's combat is the one the waiting skip takes.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.number > turn
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && matches!(e.state().turn.phase, Phase::Combat | Phase::SecondMain)
    });
    assert_eq!(engine.state().turn.phase, Phase::SecondMain);
    assert_eq!(combats_to_skip(&engine, p1), 0);
}
