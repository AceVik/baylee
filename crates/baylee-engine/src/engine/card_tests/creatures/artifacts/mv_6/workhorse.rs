//! `cards/creatures/artifacts/mv_6/workhorse.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn workhorse() -> CardIndex {
    card_index("bb4ab425-3da7-455d-b217-3d585d476113")
}

/// Workhorse — "enters with four +1/+1 counters on it" (a 0/0 becomes 4/4)
/// and "Remove a +1/+1 counter from this creature: Add {C}": each removal is
/// one colorless mana and one point smaller, and the fourth leaves a 0/0 that
/// dies.
#[test]
fn workhorse_trades_its_counters_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[workhorse()])
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let horse = cast_resolved(&mut engine, p0, workhorse());
    assert_eq!(counters_on(&engine, horse, CounterKind::P1P1), 4);
    assert_eq!(pt(&engine, horse), (4, 4));
    assert!(is_artifact_creature(&engine, horse));

    for removed in 1..=3u16 {
        activate(&mut engine, p0, workhorse(), 0);
        assert!(
            stack_is_empty(&engine),
            "a mana ability does not use the stack"
        );
        assert_eq!(counters_on(&engine, horse, CounterKind::P1P1), 4 - removed);
        assert_eq!(
            engine.state().players[0]
                .mana_pool
                .available(ManaColor::Colorless),
            u32::from(removed)
        );
    }
    assert_eq!(pt(&engine, horse), (1, 1));
    activate(&mut engine, p0, workhorse(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, workhorse()).is_some(),
        "0/0 with no counters dies"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        4
    );
}
