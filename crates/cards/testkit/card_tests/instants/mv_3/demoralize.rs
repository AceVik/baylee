//! `cards/instants/mv_3/demoralize.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "d0741810-0772-47f3-a709-4739fbf85576"
fn demoralize() -> CardIndex {
    card_index("d0741810-0772-47f3-a709-4739fbf85576")
}

fn fill_graveyard(engine: &mut Engine<RegistryLookup>, seat: PlayerId, count: usize) {
    let cards: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Library(seat))
        .iter()
        .rev()
        .take(count)
        .copied()
        .collect();
    for id in cards {
        engine
            .dev_state_mut(seat)
            .unwrap()
            .move_object(
                id,
                ZoneLocation::Graveyard(seat),
                baylee_engine::zone::ZonePosition::Top,
                baylee_engine::event::Cause::Effect,
            )
            .unwrap();
    }
}

/// Below threshold (6 cards in graveyard): Demoralize ({2}{R}) gives all
/// creatures menace until end of turn (a single blocker is refused, two
/// blockers succeed), and menace wears off next turn.
#[test]
fn demoralize_below_threshold_grants_menace_to_all_creatures_until_eot() {
    let mut engine = lk::game(
        SEED,
        &[index::HILL_GIANT],
        &[index::SERRA_ANGEL, index::GRIZZLY_BEARS],
        &[demoralize()],
        &[],
    );
    fill_graveyard(&mut engine, P0, 6);
    let giant = lk::obj(&engine, P0, index::HILL_GIANT);
    let angel = lk::obj(&engine, P1, index::SERRA_ANGEL);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Red, 3)]);
    cast_with_floating(&mut engine, P0, demoralize());
    lk::settle(&mut engine);

    assert!(keywords(&engine, giant).contains(KeywordSet::MENACE));
    assert!(keywords(&engine, angel).contains(KeywordSet::MENACE));
    assert!(!keywords(&engine, angel).contains(KeywordSet::CANT_BLOCK));

    let _ = lk::attack(&mut engine, &[giant]);
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(angel, giant)],
                },
            )
            .is_err(),
        "menace rejects a single blocker"
    );
    lk::block(&mut engine, &[(angel, giant), (bears, giant)]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    engine.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    lk::through_combat(&mut engine);

    reach_their_main_phase(&mut engine, P1);
    assert!(
        !keywords(&engine, angel).contains(KeywordSet::MENACE),
        "menace wore off at end of turn"
    );
}

/// With threshold (7+ cards in graveyard): creatures also cannot block this
/// turn (even two blockers cannot block).
#[test]
fn demoralize_with_threshold_prevents_all_blocking_this_turn() {
    let mut engine = lk::game(
        SEED,
        &[index::HILL_GIANT],
        &[index::GRIZZLY_BEARS, index::SAVANNAH_LIONS],
        &[demoralize()],
        &[],
    );
    fill_graveyard(&mut engine, P0, 7);
    let giant = lk::obj(&engine, P0, index::HILL_GIANT);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Red, 3)]);
    cast_with_floating(&mut engine, P0, demoralize());
    lk::settle(&mut engine);

    assert!(keywords(&engine, bears).contains(KeywordSet::CANT_BLOCK));
    let offer = lk::attack(&mut engine, &[giant]);
    assert!(
        offer.is_empty(),
        "threshold Demoralize prevents all creatures from blocking: {offer:?}"
    );
}
