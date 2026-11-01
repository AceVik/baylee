//! `cards/instants/mv_2/lightning_blow.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn lightning_blow() -> CardIndex {
    card_index("6b7bbdb7-bdd5-4eb1-912e-f2f810772ac8")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Lightning Blow (6b7bbdb7-bdd5-4eb1-912e-f2f810772ac8) — {1}{W} Instant:
/// - "Target creature gains first strike until end of turn."
/// - "Draw a card at the beginning of the next turn's upkeep."
#[test]
fn lightning_blow_grants_first_strike_until_end_of_turn_and_draws_on_next_turns_upkeep() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[grizzly_bears(), plains(), plains()])
        .hand(0, &[lightning_blow()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let lib_before = library_size(&engine, P0);

    cast_from_hand(&mut engine, P0, lightning_blow());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, bears).contains(KeywordSet::FIRST_STRIKE),
        "target creature gains first strike until end of turn"
    );
    assert_eq!(
        library_size(&engine, P0),
        lib_before,
        "Lightning Blow does not draw immediately upon resolution"
    );

    // Advance to the next turn's upkeep (P1's upkeep) and let the delayed trigger resolve.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert!(
        !keywords(&engine, bears).contains(KeywordSet::FIRST_STRIKE),
        "first strike expired at end of turn"
    );
    assert_eq!(
        library_size(&engine, P0),
        lib_before - 1,
        "delayed upkeep trigger drew a card for P0 at the beginning of the next turn's upkeep"
    );
}
