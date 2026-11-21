//! `cards/instants/mv_1/skeletal_scrying.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "561c68d8-b727-48d8-a832-2e42d1345fac"
fn skeletal_scrying() -> CardIndex {
    card_index("561c68d8-b727-48d8-a832-2e42d1345fac")
}

/// Skeletal Scrying ({X}{B}): X is capped by the number of cards in your
/// graveyard; exiles X cards from your graveyard as an additional cost, then
/// draws X cards and loses X life.
#[test]
fn skeletal_scrying_exiles_x_from_graveyard_draws_x_and_loses_x_life() {
    let mut engine = lk::game(SEED, &[], &[], &[skeletal_scrying(), shock(), shock()], &[]);

    // Put exactly 2 cards into P0's graveyard.
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Red, 2), (ManaColor::Black, 4)],
    );
    lk::cast_at(&mut engine, P0, shock(), &[], &[P1]);
    lk::cast_at(&mut engine, P0, shock(), &[], &[P1]);
    let gy_cards = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(P0))
        .clone();
    assert_eq!(gy_cards.len(), 2);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    cast_with_floating(&mut engine, P0, skeletal_scrying());

    let Pending::ChooseNumber { max, .. } = engine.pending().clone() else {
        panic!("expected ChooseNumber for X, got {:?}", engine.pending());
    };
    assert_eq!(
        max, 2,
        "X is capped at 2 by graveyard size even with 4 mana floating"
    );
    engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();

    while let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .unwrap();
    }
    lk::settle(&mut engine);

    assert_eq!(
        lk::pool_total(&engine, P0),
        1,
        "{{2}}{{B}} spent of 4 black"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(P0)).len(),
        2,
        "2 graveyard cards exiled as cost"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before + 1,
        "cast 1 spell, drew 2 cards -> net +1"
    );
    assert_eq!(lk::life(&engine, P0), 18, "lost 2 life");
}

/// Skeletal Scrying can be cast for X = 0 even with an empty graveyard,
/// drawing 0 and losing 0 life.
#[test]
fn skeletal_scrying_casts_for_x_zero_with_empty_graveyard() {
    let mut engine = lk::game(SEED, &[], &[], &[skeletal_scrying()], &[]);
    lk::float(&mut engine, P0, &[(ManaColor::Black, 2)]);
    cast_with_floating(&mut engine, P0, skeletal_scrying());
    if matches!(engine.pending(), Pending::ChooseNumber { .. }) {
        engine.apply(P0, PlayerAction::ChooseNumber(0)).unwrap();
    }
    lk::settle(&mut engine);

    assert_eq!(lk::life(&engine, P0), 20);
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(P0)).len(), 0);
    assert!(in_graveyard(&engine, P0, skeletal_scrying()).is_some());
}
