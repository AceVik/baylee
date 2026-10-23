#[allow(clippy::wildcard_imports)]
use super::*;

fn spellbook() -> CardIndex {
    card_index("83133f51-bfbd-4db5-9be0-f660e3b66435")
}

#[test]
fn spellbook_removes_maximum_hand_size_at_cleanup() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(
            0,
            &[
                spellbook(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(
            1,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Cast Spellbook for {0}, leaving 9 cards in hand.
    cast_from_hand(&mut engine, p0, spellbook());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, spellbook()).is_some());
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 9);

    // Pass through cleanup into opponent's turn: no cleanup discard is asked.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 9);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::DiscardChoice { .. })
    });
    let Pending::DiscardChoice { player, count } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(
        player, p1,
        "opponent still discards to their normal maximum"
    );
    assert_eq!(count, 3);
    let discarded = engine.state().zones.list(ZoneLocation::Hand(p1))[..3].to_vec();
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: discarded })
        .unwrap();
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p1)).len(), 7);
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 9);
}
