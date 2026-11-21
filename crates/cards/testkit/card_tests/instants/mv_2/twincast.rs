//! `cards/instants/mv_2/twincast.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "8f878efc-850f-43d2-a6fe-5ea8d1dd5afb"
fn twincast() -> CardIndex {
    card_index("8f878efc-850f-43d2-a6fe-5ea8d1dd5afb")
}

/// Twincast ({U}{U}): cannot target a creature spell on the stack, copies
/// target instant or sorcery spell, and lets you choose new targets for the copy.
#[test]
fn twincast_copies_instant_and_retargets_while_rejecting_creature_spell() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[],
        &[twincast()],
        &[index::GRIZZLY_BEARS, index::LIGHTNING_BOLT],
    );
    reach_their_main_phase(&mut engine, P1);

    // 1. Creature spell on the stack is not a legal target for Twincast.
    lk::float(&mut engine, P1, &[(ManaColor::Green, 2)]);
    cast_with_floating(&mut engine, P1, index::GRIZZLY_BEARS);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 2)]);
    assert!(
        !lk::castable(&engine, P0, twincast()),
        "Twincast cannot target a creature spell"
    );
    lk::settle(&mut engine);

    // 2. P1 casts Lightning Bolt at P0; P0 casts Twincast and retargets copy to P1.
    lk::float(&mut engine, P1, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, P1, index::LIGHTNING_BOLT);
    lk::aim(&mut engine, &[], &[P0]);
    let bolt = top_of_stack(&engine);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    cast_with_floating(&mut engine, P0, twincast());
    lk::aim(&mut engine, &[bolt], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(
        lk::life(&engine, P1),
        17,
        "retargeted Bolt copy dealt 3 to P1"
    );
    assert_eq!(lk::life(&engine, P0), 17, "original Bolt dealt 3 to P0");
}
