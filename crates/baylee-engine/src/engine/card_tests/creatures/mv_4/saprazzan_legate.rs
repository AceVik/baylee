//! `cards/creatures/mv_4/saprazzan_legate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "d20b62fc-e248-4c84-87ac-7319b92531d4"
fn saprazzan_legate() -> CardIndex {
    card_index("d20b62fc-e248-4c84-87ac-7319b92531d4")
}

/// `Saprazzan Legate` (`Coverage::Partial`):
/// "Flying"
///
/// Verifies that `Saprazzan Legate` can be cast for `{3}{U}` and enters the
/// battlefield as a 1/3 creature with flying.
#[test]
fn saprazzan_legate_enters_with_flying() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9614, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[baylee_core::generated::index::GRIZZLY_BEARS])
        .hand(0, &[saprazzan_legate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, saprazzan_legate());
    pass_until(&mut engine, stack_is_empty);

    let legate = on_battlefield(&engine, p0, saprazzan_legate()).expect("P0 Saprazzan Legate");
    assert_eq!(pt(&engine, legate), (1, 3));
    assert!(keywords(&engine, legate).contains(KeywordSet::FLYING));
    let p1 = PlayerId::new(1);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p1, baylee_core::generated::index::GRIZZLY_BEARS)
        .expect("the defending player has a ground blocker");
    let blocks = attack_and_collect_blocks(&mut engine, legate, p1);
    assert!(!blocks.iter().any(|option| option.blocker == bears));
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(bears, legate)],
                }
            )
            .is_err(),
        "flying prevents the Bears blocking the Legate"
    );
    pass_until(&mut engine, |e| e.state().turn.step == Step::End);
    assert_eq!(engine.state().players[1].life, 19);
}
