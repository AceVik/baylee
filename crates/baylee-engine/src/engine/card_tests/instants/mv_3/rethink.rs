//! `cards/instants/mv_3/rethink.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn rethink() -> CardIndex {
    card_index("ede61683-fa78-4103-8248-e0e64162ed49")
}

/// Rethink ({2}{U} Instant):
/// "Counter target spell unless its controller pays {X}, where X is its mana value."
#[test]
fn rethink_counters_target_spell_when_controller_declines_to_pay_its_mana_value() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let subject = rethink();
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[forest(), forest()])
        .battlefield(1, &[island(), island(), island()])
        .hand(0, &[index::GRIZZLY_BEARS])
        .hand(1, &[subject])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_from_hand(&mut engine, p0, index::GRIZZLY_BEARS);
    let spell = engine.state().zones.list(ZoneLocation::Stack)[0];

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    cast_from_hand(&mut engine, p1, subject);
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![spell],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p0),
    );
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, index::GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, p0, index::GRIZZLY_BEARS).is_some());
}

/// When the target spell's controller pays {X} equal to its mana value, Rethink does not counter it.
#[test]
fn rethink_does_not_counter_when_controller_pays_target_spells_mana_value() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let subject = rethink();
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .battlefield(1, &[island(), island(), island()])
        .hand(0, &[index::GRIZZLY_BEARS])
        .hand(1, &[subject])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_from_hand(&mut engine, p0, index::GRIZZLY_BEARS);
    let spell = engine.state().zones.list(ZoneLocation::Stack)[0];

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    cast_from_hand(&mut engine, p1, subject);
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![spell],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p0),
    );
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, index::GRIZZLY_BEARS).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}
