//! `cards/enchantments/mv_4/breath_of_dreams.rs`, played.
//!
//! Oracle:
//! - "Cumulative upkeep {U}"
//! - "Green creatures have 'Cumulative upkeep {1}.'"

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::counters;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "477f5011-aaeb-4a5f-8a65-5fb45cb06389"
fn breath_of_dreams() -> CardIndex {
    card_index("477f5011-aaeb-4a5f-8a65-5fb45cb06389")
}

#[test]
fn breath_of_dreams_grants_cumulative_upkeep_to_green_creatures_only() {
    let mut engine = lk::game(
        SEED,
        &[index::HILL_GIANT],
        &[index::GRIZZLY_BEARS, index::SAVANNAH_LIONS],
        &[breath_of_dreams()],
        &[],
    );

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 4)]);
    cast_with_floating(&mut engine, P0, breath_of_dreams());
    pass_until(&mut engine, stack_is_empty);

    // Advance to P1's upkeep: P1's green Grizzly Bears has cumulative upkeep {1},
    // while P1's white Savannah Lions does not. With no mana, Grizzly Bears is sacrificed.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine
        .apply(P1, PlayerAction::YesNo(false))
        .expect("decline cumulative upkeep {1} on Grizzly Bears");
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });

    assert!(
        on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_none(),
        "green creature is sacrificed when its granted cumulative upkeep {{1}} is unpaid"
    );
    assert!(
        on_battlefield(&engine, P1, index::SAVANNAH_LIONS).is_some(),
        "non-green creature does not gain cumulative upkeep"
    );

    // Advance to P0's upkeep: Breath of Dreams itself has cumulative upkeep {U}.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep && !stack_is_empty(e)
    });
    lk::float(&mut engine, P0, &[(ManaColor::Blue, 1)]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine
        .apply(P0, PlayerAction::YesNo(true))
        .expect("pay {U} for Breath of Dreams cumulative upkeep");
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.phase == Phase::FirstMain
    });

    let breath = on_battlefield(&engine, P0, breath_of_dreams()).expect("still on battlefield");
    assert_eq!(
        engine
            .state()
            .object(breath)
            .unwrap()
            .counters
            .get(counters::AGE),
        1
    );
}
