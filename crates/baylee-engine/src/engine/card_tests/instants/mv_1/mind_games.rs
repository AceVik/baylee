//! `cards/instants/mv_1/mind_games.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mind_games() -> CardIndex {
    card_index("ea43372f-b0a7-4d1b-be3e-a4a738b06d59")
}

/// p0 has `lands` Islands and Mind Games in hand; p1 has Grizzly Bears, two
/// Islands and a Counterspell. p0's main phase.
fn mind_games_over_a_bear(lands: usize) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &vec![island(); lands])
        .battlefield(1, &[index::GRIZZLY_BEARS, island(), island()])
        .hand(0, &[mind_games()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Mind Games (`ea43372f-b0a7-4d1b-be3e-a4a738b06d59`) — {U} Instant:
/// "Tap target artifact, creature, or land."
#[test]
fn mind_games_taps_target_artifact_creature_or_land() {
    let mut engine = mind_games_over_a_bear(1);

    let bear = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, mind_games());
    aim_at(&mut engine, P0, bear);
    pass_until(&mut engine, stack_is_empty);
    assert_not_bought_back(&engine, P0, mind_games());

    assert!(is_tapped(&engine, bear));
}

#[test]
fn artifacts_and_lands_are_legal_but_enchantments_are_not() {
    let mut e = lk::game(
        SEED,
        &[],
        &[index::SOL_RING, forest(), index::MEGRIM],
        &[mind_games(), mind_games()],
        &[],
    );
    lk::float(&mut e, P0, &[(ManaColor::Blue, 2)]);
    let enchantment = lk::obj(&e, P1, index::MEGRIM);
    for target_card in [index::SOL_RING, forest()] {
        let target = lk::obj(&e, P1, target_card);
        cast_with_floating(&mut e, P0, mind_games());
        assert!(
            e.apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![enchantment],
                    players: vec![]
                }
            )
            .is_err()
        );
        lk::aim(&mut e, &[target], &[]);
        lk::settle(&mut e);
        assert!(lk::tapped(&e, target));
    }
}

/// "Buyback {2}{U}": the {2}{U} is paid on top, and the card comes home.
#[test]
fn mind_games_with_its_buyback_paid_comes_back_to_hand() {
    let mut engine = mind_games_over_a_bear(4);
    let bear = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, mind_games());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bear);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{U}} and {{2}}{{U}} are the four floating"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, bear));
    assert_bought_back(&engine, P0, mind_games());
}

/// Declined, it costs three less and is a Mind Games that is spent.
#[test]
fn mind_games_without_its_buyback_goes_to_the_graveyard() {
    let mut engine = mind_games_over_a_bear(4);
    let bear = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, mind_games());
    answer_buyback(&mut engine, P0, false);
    aim_at(&mut engine, P0, bear);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, bear));
    assert_not_bought_back(&engine, P0, mind_games());
}

/// Countered, it does not resolve: buyback paid, card gone, nothing tapped.
#[test]
fn mind_games_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let mut engine = mind_games_over_a_bear(4);
    let bear = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, mind_games());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bear);
    counter_the_top_spell(&mut engine, P0, P1);

    assert!(!is_tapped(&engine, bear));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_not_bought_back(&engine, P0, mind_games());
}

/// With its one target gone it does not resolve either.
#[test]
fn mind_games_that_loses_its_target_goes_to_the_graveyard() {
    let mut engine = mind_games_over_a_bear(4);
    let bear = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, mind_games());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bear);
    bury(&mut engine, &[bear]);
    pass_until(&mut engine, stack_is_empty);

    assert_not_bought_back(&engine, P0, mind_games());
}
