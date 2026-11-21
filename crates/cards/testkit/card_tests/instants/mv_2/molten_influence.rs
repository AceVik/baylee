//! `cards/instants/mv_2/molten_influence.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "d955a26a-d474-4dff-9eba-69fcfe92d8ae"
fn molten_influence() -> CardIndex {
    card_index("d955a26a-d474-4dff-9eba-69fcfe92d8ae")
}

/// Molten Influence ({1}{R}): counters target instant or sorcery if its
/// controller declines 4 damage, or lets it resolve if they take 4 damage.
#[test]
fn molten_influence_counters_instant_unless_controller_takes_four_damage() {
    // 1. Declining 4 damage counters Lightning Bolt.
    let mut declined = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[molten_influence()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut declined);
    reach_their_main_phase(&mut declined, P1);

    tap_all_mana(&mut declined, P1);
    cast_with_floating(&mut declined, P1, lightning_bolt());
    lk::aim(&mut declined, &[], &[P0]);
    let bolt = on_stack(&declined, lightning_bolt()).unwrap();
    declined.apply(P1, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut declined, P0);
    cast_with_floating(&mut declined, P0, molten_influence());
    lk::aim(&mut declined, &[bolt], &[]);
    pass_until(&mut declined, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    declined.apply(P1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut declined, stack_is_empty);

    assert_eq!(lk::life(&declined, P0), 20, "Bolt was countered");
    assert_eq!(lk::life(&declined, P1), 20, "P1 declined the 4 damage");

    // 2. Accepting 4 damage lets Lightning Bolt resolve.
    let mut accepted = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[molten_influence()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut accepted);
    reach_their_main_phase(&mut accepted, P1);

    tap_all_mana(&mut accepted, P1);
    cast_with_floating(&mut accepted, P1, lightning_bolt());
    lk::aim(&mut accepted, &[], &[P0]);
    let bolt = on_stack(&accepted, lightning_bolt()).unwrap();
    accepted.apply(P1, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut accepted, P0);
    cast_with_floating(&mut accepted, P0, molten_influence());
    lk::aim(&mut accepted, &[bolt], &[]);
    pass_until(&mut accepted, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    accepted.apply(P1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut accepted, stack_is_empty);

    assert_eq!(lk::life(&accepted, P1), 16, "P1 took 4 damage");
    assert_eq!(
        lk::life(&accepted, P0),
        17,
        "Bolt resolved and dealt 3 to P0"
    );
}

/// Molten Influence cannot target a creature spell on the stack.
#[test]
fn molten_influence_cannot_target_a_creature_spell() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[molten_influence()])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    tap_all_mana(&mut engine, P1);
    cast_with_floating(&mut engine, P1, index::GRIZZLY_BEARS);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, P0);
    assert!(
        !lk::castable(&engine, P0, molten_influence()),
        "creature spell is not an instant or sorcery spell"
    );
}
