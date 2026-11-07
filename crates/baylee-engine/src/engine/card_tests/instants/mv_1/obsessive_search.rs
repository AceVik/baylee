//! `cards/instants/mv_1/obsessive_search.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn obsessive_search() -> CardIndex {
    card_index("116ee88e-9e13-4f21-bc6a-efa994b5bf75")
}

/// "Draw a card." The ordinary {U} cast draws exactly the former top card and
/// then puts the instant into its owner's graveyard.
#[test]
fn obsessive_search_costs_one_blue_and_draws_the_top_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[obsessive_search()])
        .battlefield(0, &[island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("the fixture has a library");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, obsessive_search());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(library_size(&engine, p0), library_before.len() - 1);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top),
        "the former top card is now in hand"
    );
    assert!(in_graveyard(&engine, p0, obsessive_search()).is_some());
    assert!(mine(&engine, p0, obsessive_search(), Zone::Exile).is_empty());
}

/// Madness {U} casts the exact discarded incarnation during an opponent's
/// turn. Prying Eyes supplies the real discard and leaves exactly one blue.
#[test]
fn obsessive_search_madness_draws_one_during_an_opponents_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[obsessive_search(), index::PRYING_EYES])
        .battlefield(0, &[island(); 7])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 7);

    cast_with_floating(&mut engine, p0, index::PRYING_EYES);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, min: 2, max: 2, .. } if *player == p0),
    );
    let search = in_hand(&engine, p0, obsessive_search()).unwrap();
    let other = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(p0))
        .iter()
        .copied()
        .find(|card| *card != search)
        .expect("Prying Eyes drew another card to discard");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![search, other],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::Madness { card },
                ..
            } if *player == p0 && *card == search
        )
    });
    assert_eq!(engine.state().object(search).unwrap().zone, Zone::Exile);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .expect("Prying Eyes left cards to draw");

    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().turn.active, p1, "madness ignored timing");
    assert_eq!(library_size(&engine, p0), library_before - 1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top),
        "madness drew exactly the former top card"
    );
    assert!(in_graveyard(&engine, p0, index::PRYING_EYES).is_some());
    assert_eq!(engine.state().object(search).unwrap().zone, Zone::Graveyard);
    assert!(mine(&engine, p0, obsessive_search(), Zone::Exile).is_empty());
}
