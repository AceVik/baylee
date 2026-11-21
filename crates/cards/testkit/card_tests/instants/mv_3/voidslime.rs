//! `cards/instants/mv_3/voidslime.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "c4851768-e210-4aaf-935e-715942e198f9"
fn voidslime() -> CardIndex {
    card_index("c4851768-e210-4aaf-935e-715942e198f9")
}

/// Voidslime ({G}{U}{U}): counters target spell on the stack.
#[test]
fn voidslime_counters_target_spell() {
    let mut engine = lk::game(SEED, &[], &[], &[voidslime()], &[index::GRIZZLY_BEARS]);
    reach_their_main_phase(&mut engine, P1);

    lk::float(&mut engine, P1, &[(ManaColor::Green, 2)]);
    cast_with_floating(&mut engine, P1, index::GRIZZLY_BEARS);
    let spell = top_of_stack(&engine);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Green, 1), (ManaColor::Blue, 2)],
    );
    cast_with_floating(&mut engine, P0, voidslime());
    lk::aim(&mut engine, &[spell], &[]);
    lk::settle(&mut engine);

    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_none());
}

/// Voidslime also counters both activated abilities (Prodigal Sorcerer ping)
/// and triggered abilities (Elvish Visionary ETB draw trigger).
#[test]
fn voidslime_counters_activated_and_triggered_abilities() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::PRODIGAL_SORCERER],
        &[voidslime(), voidslime()],
        &[index::ELVISH_VISIONARY],
    );
    reach_their_main_phase(&mut engine, P1);

    // 1. Counter Prodigal Sorcerer's activated ping ability.
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
    let ping = top_of_stack(&engine);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Green, 2), (ManaColor::Blue, 4)],
    );
    cast_with_floating(&mut engine, P0, voidslime());
    lk::aim(&mut engine, &[ping], &[]);
    lk::settle(&mut engine);
    assert_eq!(lk::life(&engine, P0), 20, "activated ping countered");

    // 2. Let Elvish Visionary resolve, then counter its ETB draw trigger.
    lk::float(&mut engine, P1, &[(ManaColor::Green, 2)]);
    cast_with_floating(&mut engine, P1, index::ELVISH_VISIONARY);
    pass_until(&mut engine, |e| {
        on_battlefield(e, P1, index::ELVISH_VISIONARY).is_some()
    });
    let hand1_before = engine.state().zones.list(ZoneLocation::Hand(P1)).len();
    let trigger = top_of_stack(&engine);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    cast_with_floating(&mut engine, P0, voidslime());
    lk::aim(&mut engine, &[trigger], &[]);
    lk::settle(&mut engine);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P1)).len(),
        hand1_before,
        "ETB triggered draw countered"
    );
}
