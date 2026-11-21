//! `cards/instants/mv_2/squelch.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "b8fe20ee-d52d-4494-acf4-202808a2a22d"
fn squelch() -> CardIndex {
    card_index("b8fe20ee-d52d-4494-acf4-202808a2a22d")
}

/// Squelch ({1}{U}): counters target activated ability on the stack and draws
/// a card.
#[test]
fn squelch_counters_activated_ability_and_draws_a_card() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[squelch()])
        .battlefield(1, &[index::PRODIGAL_SORCERER])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    let hand0_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    let tim = lk::obj(&engine, P1, index::PRODIGAL_SORCERER);
    engine
        .apply(
            P1,
            PlayerAction::ActivateAbility {
                source: tim,
                ability_index: 0,
            },
        )
        .unwrap();
    lk::aim(&mut engine, &[], &[P0]);
    let ability = top_of_stack(&engine);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, squelch());
    lk::aim(&mut engine, &[ability], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(lk::life(&engine, P0), 20, "activated ping was countered");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand0_before,
        "cast 1 spell, drew 1 card -> net 0"
    );
}

/// Squelch cannot target a triggered ability on the stack (activated only).
#[test]
fn squelch_cannot_target_a_triggered_ability() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[squelch()])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[index::ELVISH_VISIONARY])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    tap_all_mana(&mut engine, P1);
    cast_with_floating(&mut engine, P1, index::ELVISH_VISIONARY);
    pass_until(&mut engine, |e| {
        on_battlefield(e, P1, index::ELVISH_VISIONARY).is_some()
    });
    // ETB draw trigger is on the stack; pass priority to P0.
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, P0);
    assert!(
        !lk::castable(&engine, P0, squelch()),
        "Squelch cannot target a triggered ability"
    );
}
