//! `cards/instants/mv_5/miraculous_recovery.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn miraculous_recovery() -> CardIndex {
    card_index("cea73a4b-e10a-4517-a92b-24a08321009f")
}

/// `Miraculous Recovery` ({4}{W}, Instant, `Coverage::Implemented`):
/// `Return target creature card from your graveyard to the battlefield. Put a
/// +1/+1 counter on it.`
#[test]
fn miraculous_recovery_returns_creature_from_your_graveyard_with_p1p1_counter() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                GRIZZLY_BEARS,
            ],
        )
        .hand(0, &[miraculous_recovery()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear_on_board = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    kill(&mut engine, bear_on_board);
    engine
        .apply(PlayerId::new(1), PlayerAction::PassPriority)
        .unwrap();
    let dead_bear = in_graveyard(&engine, p0, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, miraculous_recovery());
    aim_at(&mut engine, p0, dead_bear);
    pass_until(&mut engine, stack_is_empty);

    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    assert_eq!(counters_on(&engine, bear, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, bear), (3, 3));
}
