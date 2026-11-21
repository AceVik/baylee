//! `cards/instants/mv_1/stifle.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "b3b00911-ece7-4484-bc36-f211ce72b6cc"
fn stifle() -> CardIndex {
    card_index("b3b00911-ece7-4484-bc36-f211ce72b6cc")
}

/// Stifle ({U}): counters target activated ability on the stack (Prodigal
/// Sorcerer's ping deals 0 damage).
#[test]
fn stifle_counters_target_activated_ability() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .hand(0, &[stifle()])
        .battlefield(1, &[index::PRODIGAL_SORCERER])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

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
    cast_with_floating(&mut engine, P0, stifle());
    lk::aim(&mut engine, &[ability], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        lk::life(&engine, P0),
        20,
        "activated ping ability was countered"
    );
}

/// Stifle also counters target triggered ability on the stack (Elvish
/// Visionary's ETB draw trigger), and cannot target a spell on the stack.
#[test]
fn stifle_counters_triggered_ability_and_cannot_target_a_spell() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .hand(0, &[stifle()])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[index::ELVISH_VISIONARY])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    tap_all_mana(&mut engine, P1);
    cast_with_floating(&mut engine, P1, index::ELVISH_VISIONARY);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    // While Elvish Visionary is still a SPELL on the stack, Stifle cannot be cast.
    tap_all_mana(&mut engine, P0);
    assert!(
        !lk::castable(&engine, P0, stifle()),
        "Stifle cannot target a spell on the stack"
    );
    pass_until(&mut engine, |e| {
        on_battlefield(e, P1, index::ELVISH_VISIONARY).is_some()
    });

    // Visionary resolved and put its ETB draw trigger on the stack; P1 passes to P0.
    let hand1_before = engine.state().zones.list(ZoneLocation::Hand(P1)).len();
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    let trigger = top_of_stack(&engine);
    cast_with_floating(&mut engine, P0, stifle());
    lk::aim(&mut engine, &[trigger], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, index::ELVISH_VISIONARY).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P1)).len(),
        hand1_before,
        "ETB draw trigger was countered"
    );
}
