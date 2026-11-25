//! The rollout net's seat through the evaluator's game loop, with stand-ins
//! for both nets: plumbing, not playing strength.
use super::*;
use crate::lookahead::GameSearch;
use crate::lookahead::tests::{Deal, Fixture, config, flat, lands, play};

/// A fair search of few decisions, its rollouts at once.
fn search() -> GameSearch {
    let config = config(3, 2, 4).with_fair(true).with_threads(4).unwrap();
    GameSearch::new(config, 6)
}

/// The searched game of the stand-in that likes lands, without a rollout net.
fn plain() -> (Deal, GameSearch, Fixture) {
    let mut player = Fixture::new(false, lands).lending();
    let mut search = search();
    let deal = play(&mut player, Some(&mut search));
    (deal, search, player)
}

/// The net beside itself as a rollout net, with its value followed to the
/// leaf, records the game it records without one: the same answers, worlds
/// and counts, and the same net afterwards.
#[test]
fn the_net_as_its_own_rollout_net_records_the_plain_searchs_game() {
    let (without, without_search, player) = plain();
    let mut seat = RolloutSeat::new(
        Fixture::new(false, lands).lending(),
        Fixture::new(false, lands).lending(),
        RolloutLeaf4::Follow,
        Vec::new(),
    );
    let mut with_search = search();
    let with = play(&mut seat, Some(&mut with_search));
    assert_eq!(with.utility, without.utility);
    assert_eq!(with.record, without.record, "byte for byte");
    assert!(with_search.tally.searched > 0);
    assert_eq!(with_search.events, without_search.events);
    with_search.tally.ms.clone_from(&without_search.tally.ms);
    assert_eq!(with_search.tally, without_search.tally);
    assert_eq!(seat.net.memory, player.memory);
    assert_eq!(
        seat.rollout.memory.observations, player.memory.observations,
        "the rollout net watched every view of the seat"
    );
    assert!(with_search.events.iter().all(|e| e["variants"].is_null()));
}

/// `--rollout-compare` plays the net's own search, and beside each searched
/// decision records the same root searched by the rollout net under every
/// leaf, in the same worlds; what it compared changes nothing at the table.
#[test]
fn compared_leaves_are_recorded_beside_the_nets_own_search() {
    let (without, without_search, player) = plain();
    let mut seat = RolloutSeat::new(
        Fixture::new(false, lands).lending(),
        Fixture::new(false, flat).lending(),
        RolloutLeaf4::Jump,
        RolloutLeaf4::ALL.to_vec(),
    );
    let mut with_search = search();
    let with = play(&mut seat, Some(&mut with_search));
    assert_eq!(with.record, without.record, "the net's own search answers");
    assert_eq!(seat.net.memory, player.memory);
    assert_eq!(with_search.events.len(), without_search.events.len());
    for (event, own) in with_search.events.iter().zip(&without_search.events) {
        let variants = &event["variants"];
        assert_eq!(event["worlds"], own["worlds"]);
        for leaf in RolloutLeaf4::ALL {
            assert_eq!(variants[leaf.name()]["candidates"], event["candidates"]);
            assert_eq!(variants[leaf.name()]["rounds"], event["rounds"]);
        }
        // The stand-ins answer alike and the net's critic reads the view
        // alone: only the rollout net's own value scores otherwise.
        assert_eq!(variants["follow"]["worlds"], event["worlds"]);
        assert_eq!(variants["jump"]["worlds"], event["worlds"]);
        let student = variants["student"]["worlds"].as_array().unwrap();
        assert!(
            student
                .iter()
                .flat_map(|world| world.as_array().unwrap())
                .any(|score| score.as_f64() == Some(0.5)),
            "{student:?}"
        );
    }
    assert!(
        seat.take_search_variants().is_none(),
        "each taken with its event"
    );
}

/// A rollout net searches fair worlds alone, whoever asks.
#[test]
fn a_rollout_net_searches_no_unfair_world() {
    let deck = baylee_train::housedeck::HouseDeck::named("victory").unwrap();
    let steady = baylee_core::preset::AIProfile::STEADY;
    let preset = baylee_train::selfplay::table_for(12, &[&deck, &deck], &[steady; 2], &[]);
    let mut session = Session::new(&preset).unwrap();
    let me = PlayerId::new(0);
    session.take_over(me);
    while session.needs_pump() {
        session.pump_at_most(1);
    }
    let (pending, view) = session.view_for_with_context(me).unwrap();
    let mut seat = RolloutSeat::new(
        Fixture::new(false, lands).lending(),
        Fixture::new(false, lands).lending(),
        RolloutLeaf4::Student,
        Vec::new(),
    );
    let error = seat
        .search(&session, &view, &pending, &[], config(2, 1, 2), 1, &[])
        .unwrap()
        .unwrap_err();
    assert!(error.to_string().contains("--search-fair"), "{error:#}");
}
