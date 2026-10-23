//! `cards/creatures/mv_3/merfolk_seer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "2351a9fa-17b5-41fc-93d0-34731d54f915"
fn merfolk_seer() -> CardIndex {
    card_index("2351a9fa-17b5-41fc-93d0-34731d54f915")
}

#[test]
fn merfolk_seer_dies_trigger_pays_1u_to_draw_a_card() {
    let mut engine = lk::game(
        SEED,
        &[merfolk_seer(), island(), island()],
        &[],
        &[index::LIGHTNING_BOLT],
        &[],
    );
    let seer = lk::obj(&engine, P0, merfolk_seer());

    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 1),
            (ManaColor::Blue, 1),
        ],
    );
    cast_with_floating(&mut engine, P0, index::LIGHTNING_BOLT);
    lk::aim(&mut engine, &[seer], &[]);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before + 1
    );
}

/// "You may pay": declining draws nothing.
#[test]
fn merfolk_seer_draws_nothing_when_the_controller_declines_to_pay() {
    let mut engine = lk::game(
        SEED,
        &[merfolk_seer(), island(), island()],
        &[],
        &[index::LIGHTNING_BOLT],
        &[],
    );
    let seer = lk::obj(&engine, P0, merfolk_seer());

    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 1),
            (ManaColor::Blue, 1),
        ],
    );
    cast_with_floating(&mut engine, P0, index::LIGHTNING_BOLT);
    lk::aim(&mut engine, &[seer], &[]);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before,
        "the controller declined, so no card was drawn"
    );
    assert!(
        in_graveyard(&engine, P0, merfolk_seer()).is_some(),
        "the Seer died either way"
    );
}
