//! `cards/instants/mv_2/arcane_denial.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn arcane_denial() -> CardIndex {
    card_index("ab1cc360-b9de-48d9-9983-4dfe4a7d2a37")
}

/// `Arcane Denial` ({1}{U}, Instant, `Coverage::Partial`) counters target spell
/// and draws you a card at the beginning of the next turn's upkeep.
#[test]
fn arcane_denial_counters_target_spell_and_draws_at_next_turns_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .battlefield(1, &[forest(), forest()])
        .hand(0, &[arcane_denial()])
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, grizzly_bears());
    let spell = on_stack(&engine, grizzly_bears()).expect("Grizzly Bears on stack");

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, arcane_denial());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![spell],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_none());

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}
