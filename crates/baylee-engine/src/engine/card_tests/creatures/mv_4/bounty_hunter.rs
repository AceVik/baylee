//! `cards/creatures/mv_4/bounty_hunter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn bounty_hunter() -> CardIndex {
    card_index("2fd1cfc5-fbcd-4d31-ba8b-759d8c41b435")
}

/// Bounty Hunter (2fd1cfc5-fbcd-4d31-ba8b-759d8c41b435) — {2}{B}{B} Creature — Human Archer Minion:
/// "{T}: Put a bounty counter on target nonblack creature. {T}: Destroy target creature with a bounty counter on it."
#[test]
fn bounty_hunter_marks_nonblack_creature_with_bounty_and_destroys_it() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[bounty_hunter()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    activate(&mut engine, P0, bounty_hunter(), 0);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::BOUNTY),
        1
    );

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);

    activate(&mut engine, P0, bounty_hunter(), 1);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
}
