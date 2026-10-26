//! `cards/creatures/mv_2/cloudskate.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::counters;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Cloudskate ({1}{U} 2/2 Creature — Illusion):
/// "Flying; Fading 3 (This creature enters with three fade counters on it. At the beginning of your upkeep, remove a fade counter from it. If you can't, sacrifice it.)"
#[test]
fn cloudskate_enters_with_three_fade_counters_removes_one_each_upkeep_and_sacrifices_when_empty() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[index::CLOUDSKATE])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, index::CLOUDSKATE);
    pass_until(&mut engine, stack_is_empty);

    let skate = on_battlefield(&engine, P0, index::CLOUDSKATE).unwrap();
    assert_eq!(
        engine
            .state()
            .object(skate)
            .unwrap()
            .counters
            .get(counters::FADE),
        3
    );

    // Upkeep 1 (3 -> 2), Upkeep 2 (2 -> 1), Upkeep 3 (1 -> 0): still on battlefield.
    for expected in [2, 1, 0] {
        reach_their_main_phase(&mut engine, P1);
        reach_their_main_phase(&mut engine, P0);
        assert_eq!(
            engine
                .state()
                .object(skate)
                .unwrap()
                .counters
                .get(counters::FADE),
            expected
        );
    }

    // Upkeep 4 (0 counters left): sacrificed.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert!(on_battlefield(&engine, P0, index::CLOUDSKATE).is_none());
    assert!(in_graveyard(&engine, P0, index::CLOUDSKATE).is_some());
}
