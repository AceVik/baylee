//! `cards/instants/mv_2/blessed_wine.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn blessed_wine() -> CardIndex {
    card_index("8079007c-a07a-4d71-bb08-e1dd215297de")
}

/// Blessed Wine (8079007c-a07a-4d71-bb08-e1dd215297de) — {1}{W} Instant:
/// - "You gain 1 life."
/// - "Draw a card at the beginning of the next turn's upkeep."
#[test]
fn blessed_wine_gains_one_life_immediately_and_draws_on_next_turns_upkeep() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[blessed_wine()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, blessed_wine());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 21, "gained 1 life");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        0,
        "does not draw immediately"
    );

    // Advance to P1's upkeep and resolve the delayed trigger.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        1,
        "draws 1 card at the beginning of the next turn's upkeep"
    );
}
