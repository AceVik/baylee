//! `cards/creatures/mv_3/mundungu.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn mundungu() -> CardIndex {
    card_index("3fb4161d-a5b9-411d-942c-2fdb039e7bde")
}

/// `Mundungu` ({1}{U}{B}, 1/1 Creature — Human Wizard, `Coverage::Partial`)
/// taps to counter target spell unless its controller pays `{1}`.
#[test]
fn mundungu_counters_target_spell_when_controller_cannot_pay_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), swamp()])
        .battlefield(1, &[forest(), forest()])
        .hand(0, &[mundungu()])
        .hand(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, mundungu());
    pass_until(&mut engine, stack_is_empty);

    // Advance through P1's turn and P0's next turn to P1's next main phase so Mundungu is no longer summoning-sick.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, GRIZZLY_BEARS);
    let spell = engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .copied()
        .unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    activate(&mut engine, p0, mundungu(), 0);
    aim_at(&mut engine, p0, spell);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, p1, GRIZZLY_BEARS).is_some());
}
