use super::*;
use api::ranking::{Board, Lane, Listing};

#[test]
fn ranking_read_cannot_cross_cohorts_or_resolve_an_ambiguous_publication() {
    let (mut state, cohort, _) = opened();
    let read = Read::Ranking(cohort.id, Lane::Echoes);
    let board = Board {
        cohort: cohort.id,
        lane: Lane::Echoes,
        listing: Listing::default(),
        best: None,
        leaders: Vec::new(),
    };
    let work = state.read(Uuid::now_v7(), read).unwrap();
    state.receive(
        &work,
        Some(Response::Ranking {
            board: board.clone(),
        }),
        None,
    );
    assert_eq!(state.ranking(), Some(&board));
    let publish = state.listing(Uuid::now_v7(), true).unwrap();
    state.receive(&publish, None, Some(Error::Unavailable));
    assert!(state.uncertain());
    assert!(state.listing(Uuid::now_v7(), false).is_none());
    let work = state.read(Uuid::now_v7(), read).unwrap();
    let mut changed = board.clone();
    changed.listing = Listing {
        revision: 1,
        listed: true,
    };
    state.receive(
        &work,
        Some(Response::Ranking {
            board: changed.clone(),
        }),
        None,
    );
    assert!(
        state.uncertain(),
        "a read cannot acknowledge a lost mutation response"
    );
    let retry = state.retry(Uuid::now_v7()).unwrap();
    assert_eq!(retry.request(), publish.request());
    state.receive(
        &retry,
        Some(Response::Listing {
            listing: changed.listing,
        }),
        None,
    );
    assert!(!state.uncertain());
    assert!(
        state.ranking().is_none(),
        "remove stale privacy projection until fresh read"
    );
    let work = state.read(Uuid::now_v7(), read).unwrap();
    state.receive(
        &work,
        Some(Response::Ranking {
            board: changed.clone(),
        }),
        None,
    );
    let work = state.read(Uuid::now_v7(), read).unwrap();
    let mut wrong = board;
    wrong.cohort = Uuid::now_v7();
    state.receive(&work, Some(Response::Ranking { board: wrong }), None);
    assert_eq!(state.ranking(), Some(&changed));
    assert_eq!(state.failure(), Some(Error::Unavailable));
    let work = state.listing(Uuid::now_v7(), false).unwrap();
    state.receive(&work, None, Some(Error::Conflict));
    assert!(
        !state.uncertain(),
        "stale publication is conclusively refused"
    );
}
