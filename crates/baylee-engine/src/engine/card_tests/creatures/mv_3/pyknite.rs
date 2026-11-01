//! `cards/creatures/mv_3/pyknite.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);

fn pyknite() -> CardIndex {
    card_index("857f9a8d-ba77-4afd-922d-994f66e478ef")
}

fn hand_size(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(seat)).len()
}

/// Pyknite (857f9a8d-ba77-4afd-922d-994f66e478ef) — {2}{G} 1/1 Creature — Ouphe:
/// "When this creature enters, draw a card at the beginning of the next turn's upkeep."
#[test]
fn pyknite_schedules_a_card_draw_for_the_beginning_of_the_next_turns_upkeep() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[pyknite()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let hand_before_cast = hand_size(&engine, P0);
    cast_from_hand(&mut engine, P0, pyknite());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, pyknite()).is_some());
    assert_eq!(
        hand_size(&engine, P0),
        hand_before_cast - 1,
        "Pyknite does not draw immediately upon entering"
    );

    // At the beginning of the next turn's upkeep (P1's upkeep, turn 2), P0 draws a card.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 2 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        hand_size(&engine, P0),
        hand_before_cast,
        "P0 draws a card at the beginning of the next turn's upkeep"
    );

    // Only the next turn's upkeep: P0's own upkeep two turns on (turn 3, before the draw step)
    // draws nothing extra.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        hand_size(&engine, P0),
        hand_before_cast,
        "the delayed draw happened once, at the next turn's upkeep, and not again"
    );
}
